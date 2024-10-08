use napi::Error;
use openssl::{
  asn1::Asn1Time,
  bn::{BigNum, MsbOption},
  ec::{EcGroup, EcKey},
  hash::MessageDigest,
  nid::Nid,
  pkey::PKey,
  sign::{Signer, Verifier},
  ssl::{SslConnector, SslContext, SslMethod},
  stack::Stack,
  x509::{
    extension::{AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectKeyIdentifier},
    store::X509StoreBuilder,
    X509Builder, X509NameBuilder, X509ReqBuilder, X509StoreContext, X509,
  },
};

#[napi]
pub fn generate_root_ca() -> Result<Vec<String>, Error> {
  let nid = Nid::X9_62_PRIME256V1;
  let group = EcGroup::from_curve_name(nid).unwrap();
  let private_key = EcKey::generate(&group).unwrap();

  let mut x509_builder = X509Builder::new().unwrap();
  x509_builder.set_version(2).unwrap();

  let serial_number = {
    let mut serial = BigNum::new().unwrap();
    serial.rand(159, MsbOption::MAYBE_ZERO, false).unwrap();
    serial.to_asn1_integer().unwrap()
  };
  x509_builder.set_serial_number(&serial_number).unwrap();

  let mut x509_name = X509NameBuilder::new().unwrap();
  x509_name
    .append_entry_by_nid(Nid::COMMONNAME, "Drop Root Server")
    .unwrap();
  x509_name
    .append_entry_by_nid(Nid::ORGANIZATIONNAME, "Drop")
    .unwrap();
  let x509_name_built = x509_name.build();
  x509_builder.set_subject_name(&x509_name_built).unwrap();
  x509_builder.set_issuer_name(&x509_name_built).unwrap();

  let not_before = Asn1Time::days_from_now(0).unwrap();
  x509_builder.set_not_before(&not_before).unwrap();
  let not_after = Asn1Time::days_from_now(365 * 1000).unwrap();
  x509_builder.set_not_after(&not_after).unwrap();

  x509_builder
    .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
    .unwrap();
  x509_builder
    .append_extension(
      KeyUsage::new()
        .critical()
        .key_cert_sign()
        .crl_sign()
        .digital_signature()
        .build()
        .unwrap(),
    )
    .unwrap();

  let key_pair = PKey::from_ec_key(private_key).unwrap();
  x509_builder.set_pubkey(&key_pair).unwrap();

  x509_builder
    .sign(&key_pair, MessageDigest::sha256())
    .unwrap();

  let x509 = x509_builder.build();

  return Ok(vec![
    String::from_utf8(x509.to_pem().unwrap()).unwrap(),
    String::from_utf8(key_pair.private_key_to_pem_pkcs8().unwrap()).unwrap(),
  ]);
}

#[napi]
pub fn generate_client_certificate(
  client_id: String,
  client_name: String,
  root_ca: String,
  root_ca_private: String,
) -> Result<Vec<String>, Error> {
  let root_ca_cert = X509::from_pem(root_ca.as_bytes()).unwrap();
  let root_ca_key = EcKey::private_key_from_pem(root_ca_private.as_bytes()).unwrap();
  let root_ca_key_pair = PKey::from_ec_key(root_ca_key).unwrap();

  let nid = Nid::X9_62_PRIME256V1;
  let group = EcGroup::from_curve_name(nid).unwrap();
  let private_key = EcKey::generate(&group).unwrap();
  let key_pair = PKey::from_ec_key(private_key).unwrap();

  /* Generate req and sign it */
  let mut req_builder = X509ReqBuilder::new().unwrap();
  req_builder.set_pubkey(&key_pair).unwrap();

  let mut x509_name = X509NameBuilder::new().unwrap();
  x509_name
    .append_entry_by_nid(Nid::COMMONNAME, &client_id)
    .unwrap();
  x509_name
    .append_entry_by_nid(Nid::SUBJECT_ALT_NAME, &client_name)
    .unwrap();
  x509_name
    .append_entry_by_nid(Nid::ORGANIZATIONNAME, "Drop")
    .unwrap();
  let x509_name_built = x509_name.build();

  req_builder.set_subject_name(&x509_name_built).unwrap();
  req_builder
    .sign(&key_pair, MessageDigest::sha256())
    .unwrap();
  let req = req_builder.build();

  /* Generate certificate from req and sign it using CA */
  let mut x509_builder = X509Builder::new().unwrap();
  x509_builder.set_version(2).unwrap();
  x509_builder.set_pubkey(&key_pair).unwrap();

  let serial_number = {
    let mut serial = BigNum::new().unwrap();
    serial.rand(159, MsbOption::MAYBE_ZERO, false).unwrap();
    serial.to_asn1_integer().unwrap()
  };
  x509_builder.set_serial_number(&serial_number).unwrap();

  x509_builder.set_subject_name(req.subject_name()).unwrap();
  x509_builder
    .set_issuer_name(root_ca_cert.issuer_name())
    .unwrap();

  let not_before = Asn1Time::days_from_now(0).unwrap();
  x509_builder.set_not_before(&not_before).unwrap();
  let not_after = Asn1Time::days_from_now(365 * 100).unwrap();
  x509_builder.set_not_after(&not_after).unwrap();

  x509_builder
    .append_extension(BasicConstraints::new().build().unwrap())
    .unwrap();
  x509_builder
    .append_extension(
      KeyUsage::new()
        .critical()
        .non_repudiation()
        .digital_signature()
        .data_encipherment()
        .build()
        .unwrap(),
    )
    .unwrap();

  let subject_key_identifier = SubjectKeyIdentifier::new()
    .build(&x509_builder.x509v3_context(Some(&root_ca_cert), None))
    .unwrap();
  x509_builder
    .append_extension(subject_key_identifier)
    .unwrap();

  let auth_key_identifier = AuthorityKeyIdentifier::new()
    .keyid(false)
    .issuer(false)
    .build(&x509_builder.x509v3_context(Some(&root_ca_cert), None))
    .unwrap();
  x509_builder.append_extension(auth_key_identifier).unwrap();

  x509_builder
    .sign(&root_ca_key_pair, MessageDigest::sha256())
    .unwrap();

  let x509 = x509_builder.build();

  return Ok(vec![
    String::from_utf8(x509.to_pem().unwrap()).unwrap(),
    String::from_utf8(key_pair.private_key_to_pem_pkcs8().unwrap()).unwrap(),
  ]);
}

#[napi]
pub fn verify_client_certificate(client_cert: String, root_ca: String) -> Result<bool, Error> {
  let root_ca_cert = X509::from_pem(root_ca.as_bytes()).unwrap();

  let mut store_builder = X509StoreBuilder::new().unwrap();
  store_builder.add_cert(root_ca_cert).unwrap();
  let store = store_builder.build();

  let client_cert: X509 = X509::from_pem(client_cert.as_bytes()).unwrap();

  let chain = Stack::new().unwrap();

  let mut store_ctx = X509StoreContext::new().unwrap();
  let result = store_ctx
    .init(&store, &client_cert, &chain, |c| c.verify_cert())
    .unwrap();

  return Ok(result);
}

#[napi]
pub fn sign_nonce(private_key: String, nonce: String) -> Result<String, Error> {
  let client_private_key = EcKey::private_key_from_pem(private_key.as_bytes()).unwrap();
  let pkey_private_key = PKey::from_ec_key(client_private_key).unwrap();

  let mut signer = Signer::new(MessageDigest::sha256(), &pkey_private_key).unwrap();
  signer.update(nonce.as_bytes()).unwrap();
  let signature = signer.sign_to_vec().unwrap();

  let hex_signature = hex::encode(signature);

  return Ok(hex_signature);
}

#[napi]
pub fn verify_nonce(public_cert: String, nonce: String, signature: String) -> Result<bool, Error> {
  let client_public_cert = X509::from_pem(public_cert.as_bytes()).unwrap();
  let client_public_key = client_public_cert.public_key().unwrap();

  let signature = hex::decode(signature).unwrap();

  let mut verifier = Verifier::new(
    MessageDigest::sha256(),
    &client_public_key,
  )
  .unwrap();
  verifier.update(nonce.as_bytes()).unwrap();

  let result = verifier.verify(&signature).unwrap();

  return Ok(result);
}

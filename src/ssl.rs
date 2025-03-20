use napi::Error;
use openssl::asn1::Asn1Integer;
use openssl::{
  asn1::Asn1Time,
  bn::{BigNum, MsbOption},
  ec::{EcGroup, EcKey},
  hash::MessageDigest,
  nid::Nid,
  pkey::PKey,
  sign::{Signer, Verifier},
  stack::Stack,
  x509::{
    extension::{AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectKeyIdentifier},
    store::X509StoreBuilder,
    X509Builder, X509NameBuilder, X509ReqBuilder, X509StoreContext, X509,
  },
};
use rcgen::{
  Certificate, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, Ia5String, IsCa,
  KeyPair, KeyUsagePurpose, SanType, SerialNumber,
};
use time::{Duration, OffsetDateTime};

fn create_serial_number() -> Asn1Integer {
  let mut serial = BigNum::new().unwrap();
  serial.rand(159, MsbOption::MAYBE_ZERO, false).unwrap();
  serial.to_asn1_integer().unwrap()
}

#[napi]
pub fn generate_root_ca() -> Result<Vec<String>, Error> {
  let mut params = CertificateParams::default();

  let mut name = DistinguishedName::new();
  name.push(rcgen::DnType::CommonName, "Drop Root Server");
  name.push(rcgen::DnType::OrganizationName, "Drop");

  params.distinguished_name = name;

  params.not_before = OffsetDateTime::now_utc();
  params.not_after = OffsetDateTime::now_utc()
    .checked_add(Duration::days(365 * 1000))
    .unwrap();

  params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

  params.key_usages = vec![
    KeyUsagePurpose::CrlSign,
    KeyUsagePurpose::KeyCertSign,
    KeyUsagePurpose::DigitalSignature,
  ];

  let key_pair = KeyPair::generate().map_err(|e| napi::Error::from_reason(e.to_string()))?;
  let certificate = CertificateParams::self_signed(params, &key_pair)
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

  // Returns certificate, then private key
  Ok(vec![certificate.pem(), key_pair.serialize_pem()])
}

#[napi]
pub fn generate_client_certificate(
  client_id: String,
  client_name: String,
  root_ca: String,
  root_ca_private: String,
) -> Result<Vec<String>, Error> {
  let root_key_pair =
    KeyPair::from_pem(&root_ca_private).map_err(|e| napi::Error::from_reason(e.to_string()))?;
  let certificate_params = CertificateParams::from_ca_cert_pem(&root_ca)
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;
  let root_ca = CertificateParams::self_signed(certificate_params, &root_key_pair)
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

  let mut params = CertificateParams::default();

  let mut name = DistinguishedName::new();
  name.push(rcgen::DnType::CommonName, client_id);
  name.push(rcgen::DnType::OrganizationName, "Drop");
  params.distinguished_name = name;

  params.key_usages = vec![
    KeyUsagePurpose::DigitalSignature,
    KeyUsagePurpose::DataEncipherment,
  ];

  let key_pair = KeyPair::generate().map_err(|e| napi::Error::from_reason(e.to_string()))?;
  let certificate = CertificateParams::signed_by(params, &key_pair, &root_ca, &root_key_pair)
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

  // Returns certificate, then private key
  Ok(vec![certificate.pem(), key_pair.serialize_pem()])
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

  Ok(result)
}

#[napi]
pub fn sign_nonce(private_key: String, nonce: String) -> Result<String, Error> {
  let client_private_key = EcKey::private_key_from_pem(private_key.as_bytes()).unwrap();
  let pkey_private_key = PKey::from_ec_key(client_private_key).unwrap();

  let mut signer = Signer::new(MessageDigest::sha256(), &pkey_private_key).unwrap();
  signer.update(nonce.as_bytes()).unwrap();
  let signature = signer.sign_to_vec().unwrap();

  let hex_signature = hex::encode(signature);

  Ok(hex_signature)
}

#[napi]
pub fn verify_nonce(public_cert: String, nonce: String, signature: String) -> Result<bool, Error> {
  let client_public_cert = X509::from_pem(public_cert.as_bytes()).unwrap();
  let client_public_key = client_public_cert.public_key().unwrap();

  let signature = hex::decode(signature).unwrap();

  let mut verifier = Verifier::new(MessageDigest::sha256(), &client_public_key).unwrap();
  verifier.update(nonce.as_bytes()).unwrap();

  let result = verifier.verify(&signature).unwrap();

  Ok(result)
}

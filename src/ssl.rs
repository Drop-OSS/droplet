use anyhow::anyhow;
use napi::Error;
use rcgen::{
  CertificateParams, DistinguishedName, IsCa, KeyPair, KeyUsagePurpose, PublicKeyData,
  SubjectPublicKeyInfo,
};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, VerificationAlgorithm};
use time::{Duration, OffsetDateTime};
use x509_parser::parse_x509_certificate;
use x509_parser::pem::Pem;

#[napi]
pub fn generate_root_ca() -> anyhow::Result<Vec<String>> {
  let mut params = CertificateParams::default();

  let mut name = DistinguishedName::new();
  name.push(rcgen::DnType::CommonName, "Drop Root Server");
  name.push(rcgen::DnType::OrganizationName, "Drop");

  params.distinguished_name = name;

  params.not_before = OffsetDateTime::now_utc();
  params.not_after = OffsetDateTime::now_utc()
    .checked_add(Duration::days(365 * 1000))
    .ok_or(anyhow!("failed to calculate end date"))?;

  params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

  params.key_usages = vec![
    KeyUsagePurpose::CrlSign,
    KeyUsagePurpose::KeyCertSign,
    KeyUsagePurpose::DigitalSignature,
  ];

  let key_pair = KeyPair::generate()?;
  let certificate = CertificateParams::self_signed(params, &key_pair)?;

  // Returns certificate, then private key
  Ok(vec![certificate.pem(), key_pair.serialize_pem()])
}

#[napi]
pub fn generate_client_certificate(
  client_id: String,
  _client_name: String,
  root_ca: String,
  root_ca_private: String,
) -> anyhow::Result<Vec<String>> {
  let root_key_pair = KeyPair::from_pem(&root_ca_private)?;
  let certificate_params = CertificateParams::from_ca_cert_pem(&root_ca)?;
  let root_ca = CertificateParams::self_signed(certificate_params, &root_key_pair)?;

  let mut params = CertificateParams::default();

  let mut name = DistinguishedName::new();
  name.push(rcgen::DnType::CommonName, client_id);
  name.push(rcgen::DnType::OrganizationName, "Drop");
  params.distinguished_name = name;

  params.key_usages = vec![
    KeyUsagePurpose::DigitalSignature,
    KeyUsagePurpose::DataEncipherment,
  ];

  let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P384_SHA384)?;
  let certificate = CertificateParams::signed_by(params, &key_pair, &root_ca, &root_key_pair)?;

  // Returns certificate, then private key
  Ok(vec![certificate.pem(), key_pair.serialize_pem()])
}

#[napi]
pub fn verify_client_certificate(client_cert: String, root_ca: String) -> anyhow::Result<bool> {
  let root_ca = Pem::iter_from_buffer(root_ca.as_bytes())
    .next()
    .ok_or(anyhow!("no certificates in root ca"))??;
  let root_ca = root_ca.parse_x509()?;

  let client_cert = Pem::iter_from_buffer(client_cert.as_bytes())
    .next()
    .ok_or(anyhow!("No client certs in chain."))??;
  let client_cert = client_cert.parse_x509()?;

  let valid = root_ca
    .verify_signature(Some(client_cert.public_key()))
    .is_ok();

  Ok(valid)
}

#[napi]
pub fn sign_nonce(private_key: String, nonce: String) -> anyhow::Result<String> {
  let rng = SystemRandom::new();

  let key_pair = KeyPair::from_pem(&private_key)?;

  let key_pair = EcdsaKeyPair::from_pkcs8(
    &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
    &key_pair.serialize_der(),
    &rng,
  )
  .map_err(|e| napi::Error::from_reason(e.to_string()))?;

  let signature = key_pair
    .sign(&rng, nonce.as_bytes())
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;
  let hex_signature = hex::encode(signature);

  Ok(hex_signature)
}

#[napi]
pub fn verify_nonce(public_cert: String, nonce: String, signature: String) -> anyhow::Result<bool> {
  let (_, pem) = x509_parser::pem::parse_x509_pem(public_cert.as_bytes())?;
  let (_, spki) = parse_x509_certificate(&pem.contents)?;
  let public_key = SubjectPublicKeyInfo::from_der(spki.public_key().raw)?;

  let raw_signature = hex::decode(signature)?;

  let valid = ring::signature::ECDSA_P384_SHA384_FIXED
    .verify(
      public_key.der_bytes().into(),
      nonce.as_bytes().into(),
      raw_signature[..].into(),
    )
    .is_ok();

  Ok(valid)
}

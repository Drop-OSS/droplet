use napi::Error;
use rcgen::{
  date_time_ymd, generate_simple_self_signed, BasicConstraints, Certificate, CertificateParams,
  DistinguishedName, DnType, Ia5String, IsCa, KeyIdMethod, KeyPair, KeyUsagePurpose, SanType,
  PKCS_ECDSA_P384_SHA384,
};
use time::{Duration, OffsetDateTime};
use x509_verify::der::DecodePem;

const YEAR: i64 = 60 * 60 * 24 * 365;

#[napi]
pub fn generate_root_ca() -> Result<Vec<String>, Error> {
  let mut params = CertificateParams::new(Vec::new()).unwrap();

  let mut name = DistinguishedName::new();
  name.push(DnType::CommonName, "Drop Root CA");
  name.push(DnType::OrganizationName, "Drop");

  params.distinguished_name = name;
  params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
  params.key_usages = vec![
    KeyUsagePurpose::DigitalSignature,
    KeyUsagePurpose::KeyCertSign,
  ];

  let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384).unwrap();

  let root_ca = CertificateParams::self_signed(params, &key_pair).unwrap();

  return Ok(vec![
    key_pair.serialize_pem(),
    key_pair.public_key_pem(),
    root_ca.pem(),
  ]);
}

#[napi]
pub fn generate_client_certificate(
  client_id: String,
  client_name: String,
  root_ca: String,
  root_ca_private: String,
) -> Result<Vec<String>, Error> {
  let root_key_pair = KeyPair::from_pem(&root_ca_private).unwrap();
  let root_ca_params = CertificateParams::from_ca_cert_pem(&root_ca).unwrap();
  let root_ca = root_ca_params.self_signed(&root_key_pair).unwrap();

  let mut params = CertificateParams::new(Vec::new()).unwrap();

  let mut name = DistinguishedName::new();
  name.push(DnType::CommonName, client_id);
  name.push(DnType::OrganizationName, "Drop");

  params.distinguished_name = name;
  params.subject_alt_names = vec![SanType::DnsName(Ia5String::try_from(client_name).unwrap())];
  params.key_usages = vec![
    KeyUsagePurpose::DigitalSignature,
    KeyUsagePurpose::KeyCertSign,
  ];

  let client_key_pair = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384).unwrap();
  let client_certificate = params
    .signed_by(&client_key_pair, &root_ca, &root_key_pair)
    .unwrap();

  return Ok(vec![
    root_key_pair.serialize_pem(),
    root_key_pair.public_key_pem(),
    client_certificate.pem(),
  ]);
}

#[napi]
pub fn verify_client_certificate(client_cert: String, root_ca: String) -> Result<bool, Error> {
  let ca = x509_verify::x509_cert::Certificate::from_pem(root_ca).unwrap();
  let client = x509_verify::x509_cert::Certificate::from_pem(client_cert).unwrap();

  let key = x509_verify::VerifyingKey::try_from(&ca).unwrap();
  return Ok(match key.verify(&client) {
    Ok(_) => true,
    Err(_) => false,
  });
}

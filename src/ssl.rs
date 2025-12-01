#[napi]
pub fn generate_root_ca() -> anyhow::Result<Vec<String>> {
  Ok(droplet_rs::ssl::generate_root_ca()?)
}

#[napi]
pub fn generate_client_certificate(
  client_id: String,
  client_name: String,
  root_ca: String,
  root_ca_private: String,
) -> anyhow::Result<Vec<String>> {
  Ok(droplet_rs::ssl::generate_client_certificate(client_id, client_name, root_ca, root_ca_private)?)
}

#[napi]
pub fn verify_client_certificate(client_cert: String, root_ca: String) -> anyhow::Result<bool> {
  Ok(droplet_rs::ssl::verify_client_certificate(client_cert, root_ca)?)
}

#[napi]
pub fn sign_nonce(private_key: String, nonce: String) -> anyhow::Result<String> {
  Ok(droplet_rs::ssl::sign_nonce(private_key, nonce)?)
}

#[napi]
pub fn verify_nonce(public_cert: String, nonce: String, signature: String) -> anyhow::Result<bool> {
  Ok(droplet_rs::ssl::verify_nonce(public_cert, nonce, signature)?)
}

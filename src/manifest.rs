use std::{collections::HashMap, fs::File, path::Path};

use ciborium::into_writer;

#[derive(serde::Serialize)]
pub struct ManifestChunk {
  pub uuid: String,
  pub index: i64,
}

#[derive(serde::Serialize)]
pub struct ManifestRecord {
  pub chunks: Vec<ManifestChunk>,
  pub permissions: u32,
}

#[derive(serde::Serialize)]
pub struct Manifest {
  pub record: HashMap<String, ManifestRecord>,
}

pub fn generate_manifest(manifest: Manifest, path: &Path) {
  let file = File::create(path).unwrap();
  into_writer(&manifest, file).unwrap();
}

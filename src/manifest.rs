use std::{
  fs::File,
  io::{BufRead, BufReader},
  path::Path,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use gxhash::gxhash128;
use napi::Error;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::file_utils::list_files;

const CHUNK_SIZE: usize = 1024 * 1024 * 16;

#[derive(Serialize)]
struct Chunk {
  id: String,
  permissions: u32,
  file_name: String,
  chunk_index: u32,
  checksum: String,
}

#[napi]
pub fn generate_manifest(dir: String) -> Result<String, Error> {
  let base_dir = Path::new(&dir);
  let files = list_files(base_dir);

  let mut chunks: Vec<Chunk> = Vec::new();

  for file_path in files {
    let file = File::open(file_path.clone()).unwrap();
    let relative = file_path.strip_prefix(base_dir).unwrap();
    let permission_object = file.try_clone().unwrap().metadata().unwrap().permissions();
    let permissions = {
      let mut perm = 0;
      #[cfg(unix)]
      {
        perm = permission_object.mode();
      }
      perm
    };

    let mut reader = BufReader::with_capacity(CHUNK_SIZE, file);

    let mut chunk_index = 0;
    loop {
      let mut buffer: Vec<u8> = Vec::new();
      reader.fill_buf().unwrap().clone_into(&mut buffer);
      let length = buffer.len();

      if length == 0 {
        break;
      }

      let chunk_id = Uuid::new_v4();
      let checksum = gxhash128(&buffer, 0);
      let checksum_string = hex::encode(checksum.to_le_bytes());

      let chunk = Chunk {
        id: chunk_id.to_string(),
        chunk_index: chunk_index,
        permissions: permissions,
        file_name: relative.to_str().unwrap().to_string(),
        checksum: checksum_string,
      };

      chunks.push(chunk);

      println!("Processed chunk {} for {}", chunk_index, relative.to_str().unwrap());
      reader.consume(length);
      chunk_index += 1;

    }
  }

  Ok(json!(chunks).to_string())
}

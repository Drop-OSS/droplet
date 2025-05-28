use std::{
  collections::HashMap, fs::File, io::{BufRead, BufReader}, path::Path, rc::Rc, sync::Arc, thread
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use napi::{
  bindgen_prelude::Function,
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
  Env, Error, Result,
};
use serde_json::json;
use uuid::Uuid;

use crate::file_utils::create_backend_for_path;

const CHUNK_SIZE: usize = 1024 * 1024 * 64;

#[derive(serde::Serialize)]
struct ChunkData {
  permissions: u32,
  ids: Vec<String>,
  checksums: Vec<String>,
  lengths: Vec<usize>,
}

#[napi]
pub fn call_alt_thread_func(tsfn: Arc<ThreadsafeFunction<()>>) -> Result<(), String> {
  let tsfn_cloned = tsfn.clone();
  thread::spawn(move || {
    tsfn_cloned.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
  });
  Ok(())
}

#[napi]
pub fn generate_manifest(
  dir: String,
  progress_sfn: ThreadsafeFunction<i32>,
  log_sfn: ThreadsafeFunction<String>,
  callback_sfn: ThreadsafeFunction<String>,
) -> Result<(), String> {
  thread::spawn(move || {
    let base_dir = Path::new(&dir);
    let backend = create_backend_for_path(base_dir).unwrap();
    let files = backend.list_files(base_dir);

    // Filepath to chunk data
    let mut chunks: HashMap<String, ChunkData> = HashMap::new();

    let total: i32 = files.len() as i32;
    let mut i: i32 = 0;

    for version_file in files {
      let mut raw_reader= backend.reader(&version_file).unwrap();
      let mut reader = BufReader::with_capacity(CHUNK_SIZE, raw_reader);

      let mut chunk_data = ChunkData {
        permissions: version_file.permission,
        ids: Vec::new(),
        checksums: Vec::new(),
        lengths: Vec::new(),
      };

      let mut chunk_index = 0;
      loop {
        let mut buffer: Vec<u8> = Vec::new();
        reader.fill_buf().unwrap().clone_into(&mut buffer);
        let length = buffer.len();

        if length == 0 {
          break;
        }

        let chunk_id = Uuid::new_v4();
        let checksum = md5::compute(buffer).0;
        let checksum_string = hex::encode(checksum);

        chunk_data.ids.push(chunk_id.to_string());
        chunk_data.checksums.push(checksum_string);
        chunk_data.lengths.push(length);

        let log_str = format!(
          "Processed chunk {} for {}",
          chunk_index, &version_file.relative_filename
        );
        log_sfn.call(Ok(log_str), ThreadsafeFunctionCallMode::Blocking);

        reader.consume(length);
        chunk_index += 1;
      }

      chunks.insert(version_file.relative_filename, chunk_data);

      i += 1;
      let progress = i * 100 / total;
      progress_sfn.call(Ok(progress), ThreadsafeFunctionCallMode::Blocking);
    }

    callback_sfn.call(
      Ok(json!(chunks).to_string()),
      ThreadsafeFunctionCallMode::Blocking,
    );
  });

  Ok(())
}

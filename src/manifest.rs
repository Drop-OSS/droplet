use std::{collections::HashMap, sync::Arc, thread};

use napi::{
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
  Result,
};
use serde_json::json;
use uuid::Uuid;

use crate::version::{types::VersionBackend, utils::DropletHandler};

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
pub fn generate_manifest<'a>(
  droplet_handler: &mut DropletHandler,
  dir: String,
  progress_sfn: ThreadsafeFunction<i32>,
  log_sfn: ThreadsafeFunction<String>,
  callback_sfn: ThreadsafeFunction<String>,
) -> anyhow::Result<()> {
  let backend: &mut Box<dyn VersionBackend + Send> = droplet_handler
    .create_backend_for_path(dir)
    .ok_or(napi::Error::from_reason(
    "Could not create backend for path.",
  ))?;

  // This is unsafe (obviously)
  // But it's allg as long the DropletHandler doesn't get
  // dropped while we're generating the manifest.
  let backend: &'static mut Box<dyn VersionBackend + Send> =
    unsafe { std::mem::transmute(backend) };

  thread::spawn(move || {
    let callback_borrow = &callback_sfn;

    let mut inner = move || -> Result<()> {
      let files = backend.list_files()?;

      // Filepath to chunk data
      let mut chunks: HashMap<String, ChunkData> = HashMap::new();

      let total: i32 = files.len() as i32;
      let mut i: i32 = 0;

      let mut buf = [0u8; 1024 * 16];

      for version_file in files {
        let mut reader = backend.reader(&version_file, 0, 0)?;

        let mut chunk_data = ChunkData {
          permissions: version_file.permission,
          ids: Vec::new(),
          checksums: Vec::new(),
          lengths: Vec::new(),
        };

        let mut chunk_index = 0;
        loop {
          let mut length = 0;
          let mut buffer: Vec<u8> = Vec::new();
          let mut file_empty = false;

          loop {
            let read = reader.read(&mut buf)?;

            length += read;

            // If we're out of data, add this chunk and then move onto the next file
            if read == 0 {
              file_empty = true;
              break;
            }

            buffer.extend_from_slice(&buf[0..read]);

            if length >= CHUNK_SIZE {
              break;
            }
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

          chunk_index += 1;

          if file_empty {
            break;
          }
        }

        chunks.insert(version_file.relative_filename, chunk_data);

        i += 1;
        let progress = i * 100 / total;
        progress_sfn.call(Ok(progress), ThreadsafeFunctionCallMode::Blocking);
      }

      callback_borrow.call(
        Ok(json!(chunks).to_string()),
        ThreadsafeFunctionCallMode::Blocking,
      );

      Ok(())
    };

    let result = inner();
    if let Err(generate_err) = result {
      callback_borrow.call(Err(generate_err), ThreadsafeFunctionCallMode::Blocking);
    }
  });

  Ok(())
}

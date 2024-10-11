use std::{
  fs::File,
  io::{BufRead, BufReader},
  path::Path,
  thread,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use gxhash::gxhash128;
use napi::{
  threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
  Error, JsFunction,
};
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
pub fn call_alt_thread_func(callback: JsFunction) -> Result<(), Error> {
  let tsfn: ThreadsafeFunction<u32, ErrorStrategy::CalleeHandled> = callback
    .create_threadsafe_function(0, |ctx| {
      ctx.env.create_uint32(ctx.value + 1).map(|v| vec![v])
    })?;
  let tsfn = tsfn.clone();
  thread::spawn(move || {
    tsfn.call(Ok(0), ThreadsafeFunctionCallMode::NonBlocking);
  });
  Ok(())
}

#[napi]
pub fn generate_manifest(
  dir: String,
  progress: JsFunction,
  log: JsFunction,
  callback: JsFunction,
) -> Result<(), Error> {
  let progress_sfn: ThreadsafeFunction<i32, ErrorStrategy::CalleeHandled> = progress
    .create_threadsafe_function(0, |ctx| ctx.env.create_int32(ctx.value).map(|v| vec![v]))
    .unwrap();
  let log_sfn: ThreadsafeFunction<String, ErrorStrategy::CalleeHandled> = log
    .create_threadsafe_function(0, |ctx| ctx.env.create_string_from_std(ctx.value).map(|v| vec![v]))
    .unwrap();
  let callback_sfn: ThreadsafeFunction<String, ErrorStrategy::CalleeHandled> = callback
    .create_threadsafe_function(0, |ctx| ctx.env.create_string_from_std(ctx.value).map(|v| vec![v]))
    .unwrap();

  thread::spawn(move || {
    let base_dir = Path::new(&dir);
    let files = list_files(base_dir);

    let mut chunks: Vec<Chunk> = Vec::new();

    let total: i32 = files.len() as i32;
    let mut i: i32 = 0;

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

        let log_str = format!(
          "Processed chunk {} for {}",
          chunk_index,
          relative.to_str().unwrap()
        );
        log_sfn.call(Ok(log_str), ThreadsafeFunctionCallMode::Blocking);

        reader.consume(length);
        chunk_index += 1;
      }

      i += 1;
      let progress = i * 100 / total;
      progress_sfn.call(Ok(progress), ThreadsafeFunctionCallMode::Blocking);
    }

    callback_sfn.call(
      Ok(json!(chunks).to_string()),
      ThreadsafeFunctionCallMode::Blocking,
    );
  });

  return Ok(());
}

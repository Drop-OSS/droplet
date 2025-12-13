use std::{collections::HashMap, sync::Arc, thread};

use droplet_rs::versions::types::VersionFile;
use hashing_reader::HashingReader;
use hex::ToHex;
use humansize::{format_size, BINARY};
use napi::{
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
  Result,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt as _;
use uuid::Uuid;

use crate::version::create_backend_for_path;

const CHUNK_SIZE: u64 = 1024 * 1024 * 64;
const WIGGLE: u64 = 1024 * 1024 * 1;

#[derive(Serialize)]
struct FileEntry {
  filename: String,
  start: usize,
  length: usize,
  permissions: u32,
}

#[derive(Serialize)]
struct ChunkData {
  files: Vec<FileEntry>,
  checksum: String,
}

#[napi]
pub fn call_alt_thread_func(tsfn: Arc<ThreadsafeFunction<()>>) -> Result<(), String> {
  let tsfn_cloned = tsfn.clone();
  thread::spawn(move || {
    tsfn_cloned.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
  });
  Ok(())
}

#[derive(Serialize)]
struct Manifest {
  version: String,
  chunks: HashMap<String, ChunkData>,
  size: u64,
}

#[napi]
pub async fn generate_manifest(
  dir: String,
  progress_sfn: ThreadsafeFunction<f32>,
  log_sfn: ThreadsafeFunction<String>,
) -> anyhow::Result<String> {
  let mut backend = create_backend_for_path(dir).ok_or(napi::Error::from_reason(
    "Could not create backend for path.",
  ))?;

  let required_single_file = true; //backend.require_whole_files();

  let mut files = backend.list_files().await?;
  // Filepath to chunk data
  let mut chunks: Vec<Vec<(VersionFile, u64, u64)>> = Vec::new();
  let mut current_chunk: Vec<(VersionFile, u64, u64)> = Vec::new();

  log_sfn.call(
    Ok(format!("organizing files into chunks...",)),
    ThreadsafeFunctionCallMode::NonBlocking,
  );

  for version_file in files {
    //    let mut reader = backend.reader(&version_file, 0, 0).await?;

    // If we need the whole file, and this file would take up a whole chunk, add it to it's own chunk and move on
    if required_single_file && version_file.size >= CHUNK_SIZE {
      let size = version_file.size;
      chunks.push(vec![(version_file, 0, size)]);

      continue;
    }

    let mut current_size = current_chunk.iter().map(|v| v.2 - v.1).sum::<u64>();

    // If we need the whole file, add this current file and move on, potentially adding and creating new chunk if need be
    if required_single_file {
      let size = version_file.size.try_into().unwrap();
      current_chunk.push((version_file, 0, size));

      current_size += size;

      if current_size >= CHUNK_SIZE {
        // Pop current and add, then reset
        let new_chunk = std::mem::replace(&mut current_chunk, Vec::new());
        chunks.push(new_chunk);
      }

      continue;
    }

    // Otherwise we calculate how much of the file we need, then use that much
    let remaining_budget = (CHUNK_SIZE + WIGGLE) - current_size;
    if version_file.size >= remaining_budget {
      let remaining_budget = CHUNK_SIZE - current_size;
      current_chunk.push((version_file.clone(), 0, remaining_budget));

      let new_chunk = std::mem::replace(&mut current_chunk, Vec::new());
      chunks.push(new_chunk);

      let remaining_size = version_file.size - remaining_budget;
      let mut running_offset = remaining_budget;
      // Do everything but the last one
      while running_offset < remaining_size {
        let chunk_size = CHUNK_SIZE.min(remaining_size);
        let chunk = vec![(version_file.clone(), running_offset, chunk_size)];
        if chunk_size == CHUNK_SIZE {
          chunks.push(chunk);
        } else {
          current_chunk = chunk;
        }
        running_offset += chunk_size;
      }

      continue;
    } else {
      let size = version_file.size;
      current_chunk.push((version_file, 0, size));
      current_size += size;
    }

    if current_size >= CHUNK_SIZE {
      // Pop current and add, then reset
      let new_chunk = std::mem::replace(&mut current_chunk, Vec::new());
      chunks.push(new_chunk);
    }
  }
  if current_chunk.len() > 0 {
    chunks.push(current_chunk);
  }

  log_sfn.call(
    Ok(format!(
      "organized into {} chunks, generating checksums...",
      chunks.len()
    )),
    ThreadsafeFunctionCallMode::Blocking,
  );

  let mut manifest: HashMap<String, ChunkData> = HashMap::new();
  let mut total_manifest_length = 0;

  let mut read_buf = vec![0; 1024 * 1024 * 64];

  let chunk_len = chunks.len();
  for (index, chunk) in chunks.into_iter().enumerate() {
    let uuid = uuid::Uuid::new_v4().to_string();
    let mut hasher = Sha256::new();

    let mut chunk_data = ChunkData {
      files: Vec::new(),
      checksum: String::new(),
    };

    let mut chunk_length = 0;

    for (file, start, length) in chunk {
      log_sfn.call(
        Ok(format!(
          "reading {} from {} to {}, {}",
          file.relative_filename,
          start,
          start + length,
          format_size(length, BINARY)
        )),
        ThreadsafeFunctionCallMode::Blocking,
      );
      let mut reader = backend.reader(&file, start, start + length).await?;

      loop {
        let amount = reader.read(&mut read_buf).await?;
        if amount == 0 {
          break;
        }
        hasher.update(&read_buf[0..amount]);
      }

      chunk_length += length;

      chunk_data.files.push(FileEntry {
        filename: file.relative_filename,
        start: start.try_into().unwrap(),
        length: length.try_into().unwrap(),
        permissions: file.permission,
      });
    }

    log_sfn.call(
      Ok(format!(
        "created chunk of size {} ({}/{})",
        format_size(chunk_length, BINARY),
        index,
        chunk_len
      )),
      ThreadsafeFunctionCallMode::Blocking,
    );
    total_manifest_length += chunk_length;

    let hash: String = hasher.finalize().encode_hex();
    chunk_data.checksum = hash;
    manifest.insert(uuid, chunk_data);

    let progress: f32 = (index as f32 / chunk_len as f32) * 100.0f32;
    progress_sfn.call(Ok(progress), ThreadsafeFunctionCallMode::Blocking);
  }

  Ok(
    json!(Manifest {
      version: "2".to_string(),
      chunks: manifest,
      size: total_manifest_length
    })
    .to_string(),
  )
}

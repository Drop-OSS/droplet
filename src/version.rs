use std::{
  collections::HashMap,
  fs::File,
  path::Path,
  process::{Command, ExitStatus},
};

use anyhow::anyhow;
use droplet_rs::versions::{
  create_backend_constructor,
  types::{VersionBackend, VersionFile},
};
use napi::{
  bindgen_prelude::*,
  sys::napi_value__,
  tokio_stream::{wrappers::ReceiverStream, StreamExt},
};
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::codec::{BytesCodec, FramedRead};

pub fn create_backend_for_path(path: String) -> Option<Box<dyn VersionBackend + Send>> {
  let fs_path = Path::new(&path);
  let constructor = create_backend_constructor(fs_path)?;

  Some(constructor().ok()?)
}

#[napi]
pub fn has_backend_for_path(path: String) -> bool {
  let path = Path::new(&path);

  let has_backend = create_backend_constructor(path).is_some();

  has_backend
}

#[napi]
pub async fn list_files(path: String) -> Result<Vec<String>> {
  let mut backend =
    create_backend_for_path(path).ok_or(napi::Error::from_reason("No backend for path"))?;
  let files = backend.list_files().await?;
  Ok(files.into_iter().map(|e| e.relative_filename).collect())
}

#[napi]
pub async fn peek_file(path: String, sub_path: String) -> Result<u64> {
  let mut backend =
    create_backend_for_path(path).ok_or(napi::Error::from_reason("No backend for path"))?;

  let file = backend.peek_file(sub_path).await?;

  Ok(file.size)
}

#[napi]
pub fn read_file(
  path: String,
  sub_path: String,
  env: &Env,
  start: Option<BigInt>,
  end: Option<BigInt>,
) -> anyhow::Result<ReadableStream<BufferSlice>> {
  let mut backend = create_backend_for_path(path).ok_or(anyhow!("Failed to create backend."))?;
  let version_file = VersionFile {
    relative_filename: sub_path,
    permission: 0, // Shouldn't matter
    size: 0,       // Shouldn't matter
  };

  let (tx, rx) = tokio::sync::mpsc::channel(100);

  spawn(async move {
    // Use `?` operator for cleaner error propagation from `Option`
    let reader = backend
      .reader(
        &version_file,
        start.map(|e| e.get_u64().1).unwrap_or(0),
        end.map(|e| e.get_u64().1).unwrap_or(0),
      )
      .await
      .expect("failed to open file");

    let mut reader = BufReader::new(reader);

    let mut read_buf = [0u8; 4096];

    loop {
      let amount = reader.read(&mut read_buf).await;
      if amount.is_err() {
        let _ = tx.send(Err(napi::Error::from_reason(
          amount.unwrap_err().to_string(),
        ))).await;
        break;
      }
      let amount = amount.unwrap();
      if amount == 0 {
        break;
      }
      tx.send(Ok(read_buf[0..amount].to_vec())).await.expect("failed to send data");
    }
  });

  return Ok(ReadableStream::create_with_stream_bytes(
    env,
    ReceiverStream::new(rx),
  )?);
}

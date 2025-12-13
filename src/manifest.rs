use std::{collections::HashMap, path::PathBuf, sync::Arc, thread};

use anyhow::anyhow;
use droplet_rs::{manifest::generate_manifest_rusty, versions::types::VersionFile};
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

#[napi]
pub fn call_alt_thread_func(tsfn: Arc<ThreadsafeFunction<()>>) -> Result<(), String> {
  let tsfn_cloned = tsfn.clone();
  thread::spawn(move || {
    tsfn_cloned.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
  });
  Ok(())
}

#[napi]
pub async fn generate_manifest(
  dir: String,
  progress_sfn: ThreadsafeFunction<f32>,
  log_sfn: ThreadsafeFunction<String>,
) -> anyhow::Result<String> {
  generate_manifest_rusty(
    &PathBuf::from(dir),
    |progress| {
      progress_sfn.call(Ok(progress), ThreadsafeFunctionCallMode::Blocking);
    },
    |logline| {
      log_sfn.call(Ok(logline), ThreadsafeFunctionCallMode::Blocking);
    },
  )
  .await
}
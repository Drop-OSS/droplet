use std::{path::PathBuf, sync::Arc, thread};

use droplet_rs::manifest::generate_manifest_rusty;
use napi::{
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
  Result,
};
use serde_json::json;


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
  Ok(json!(generate_manifest_rusty(
    &PathBuf::from(dir),
    |progress| {
      progress_sfn.call(Ok(progress), ThreadsafeFunctionCallMode::Blocking);
    },
    |logline| {
      log_sfn.call(Ok(logline), ThreadsafeFunctionCallMode::Blocking);
    },
  )
  .await?).to_string())
}
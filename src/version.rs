use std::{
  collections::HashMap,
  fs::File,
  path::Path,
  process::{Command, ExitStatus},
};

use anyhow::anyhow;
use droplet_rs::versions::{create_backend_constructor, types::{ReadToAsyncRead, VersionBackend, VersionFile}};
use napi::{bindgen_prelude::*, sys::napi_value__, tokio_stream::StreamExt};
use tokio_util::codec::{BytesCodec, FramedRead};


/**
 * Persistent object so we can cache things between commands
 */
#[napi(js_name = "DropletHandler")]
pub struct DropletHandler<'a> {
  backend_cache: HashMap<String, Box<dyn VersionBackend + Send + 'a>>,
}

#[napi]
impl<'a> DropletHandler<'a> {
  #[napi(constructor)]
  pub fn new() -> Self {
    DropletHandler {
      backend_cache: HashMap::new(),
    }
  }

  pub fn create_backend_for_path(
    &mut self,
    path: String,
  ) -> Option<&mut Box<dyn VersionBackend + Send + 'a>> {
    let fs_path = Path::new(&path);
    let constructor = create_backend_constructor(fs_path)?;

    let existing_backend = match self.backend_cache.entry(path) {
      std::collections::hash_map::Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
      std::collections::hash_map::Entry::Vacant(vacant_entry) => {
        let backend = constructor().ok()?;
        vacant_entry.insert(backend)
      }
    };

    Some(existing_backend)
  }

  #[napi]
  pub fn has_backend_for_path(&self, path: String) -> bool {
    let path = Path::new(&path);

    let has_backend = create_backend_constructor(path).is_some();

    has_backend
  }

  #[napi]
  pub fn list_files(&mut self, path: String) -> Result<Vec<String>> {
    let backend = self
      .create_backend_for_path(path)
      .ok_or(napi::Error::from_reason("No backend for path"))?;
    let files = backend.list_files()?;
    Ok(files.into_iter().map(|e| e.relative_filename).collect())
  }

  #[napi]
  pub fn peek_file(&mut self, path: String, sub_path: String) -> Result<u64> {
    let backend = self
      .create_backend_for_path(path)
      .ok_or(napi::Error::from_reason("No backend for path"))?;

    let file = backend.peek_file(sub_path)?;

    Ok(file.size)
  }

  #[napi(ts_return_type = "ReadableStream")]
  pub fn read_file(
    &mut self,
    reference: Reference<DropletHandler<'static>>,
    path: String,
    sub_path: String,
    env: Env,
    start: Option<BigInt>,
    end: Option<BigInt>,
  ) -> anyhow::Result<*mut napi_value__> {
    let stream = reference.share_with(env, |handler| {
      let backend = handler
        .create_backend_for_path(path)
        .ok_or(anyhow!("Failed to create backend."))?;
      let version_file = VersionFile {
        relative_filename: sub_path,
        permission: 0, // Shouldn't matter
        size: 0,       // Shouldn't matter
      };
      // Use `?` operator for cleaner error propagation from `Option`
      let reader = backend.reader(
        &version_file,
        start.map(|e| e.get_u64().1).unwrap_or(0),
        end.map(|e| e.get_u64().1).unwrap_or(0),
      )?;

      let async_reader = ReadToAsyncRead { inner: reader };

      // Create a FramedRead stream with BytesCodec for chunking
      let stream = FramedRead::new(async_reader, BytesCodec::new())
        // Use StreamExt::map to transform each Result item
        .map(|result_item| {
          result_item
            // Apply Result::map to transform Ok(BytesMut) to Ok(Vec<u8>)
            .map(|bytes| bytes.to_vec())
            // Apply Result::map_err to transform Err(std::io::Error) to Err(napi::Error)
            .map_err(napi::Error::from) // napi::Error implements From<tokio::io::Error>
        });
      ReadableStream::create_with_stream_bytes(&env, stream)
    })?;

    Ok(stream.raw())
  }
}

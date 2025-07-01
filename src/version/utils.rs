use std::{
  fs::{self, metadata, File},
  io::Read,
  path::{Path, PathBuf},
};

use napi::{bindgen_prelude::*, tokio_stream::StreamExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use zip::ZipArchive;

use crate::version::{
  backends::{PathVersionBackend, ZipVersionBackend},
  types::{MinimumFileObject, ReadToAsyncRead, VersionBackend, VersionFile},
};

pub fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) {
  if metadata(path).unwrap().is_dir() {
    let paths = fs::read_dir(path).unwrap();
    for path_result in paths {
      let full_path = path_result.unwrap().path();
      if metadata(&full_path).unwrap().is_dir() {
        _list_files(vec, &full_path);
      } else {
        vec.push(full_path);
      }
    }
  }
}

pub fn create_backend_for_path(path: &Path) -> Option<Box<(dyn VersionBackend + Send)>> {
  let is_directory = path.is_dir();
  if is_directory {
    return Some(Box::new(PathVersionBackend {
      base_dir: path.to_path_buf(),
    }));
  };

  /*
    Insert checks for whatever backend you like
  */

  if path.ends_with(".zip") {
    return Some(Box::new(ZipVersionBackend::new(path.to_path_buf())));
  }

  None
}

#[napi]
pub fn has_backend_for_path(path: String) -> bool {
  let path = Path::new(&path);

  let has_backend = create_backend_for_path(path).is_some();

  has_backend
}

#[napi]
pub fn list_files(path: String) -> Vec<String> {
  let path = Path::new(&path);
  let mut backend = create_backend_for_path(path).unwrap();
  let files = backend.list_files();
  files.into_iter().map(|e| e.relative_filename).collect()
}

#[napi]
pub fn read_file(
  path: String,
  sub_path: String,
  env: &Env,
  start: Option<u32>,
  end: Option<u32>,
) -> Option<ReadableStream<'static, BufferSlice<'static>>> {
  let path = Path::new(&path);
  let mut backend = create_backend_for_path(path).unwrap();
  let version_file = VersionFile {
    relative_filename: sub_path,
    permission: 0, // Shouldn't matter
  };
  // Use `?` operator for cleaner error propagation from `Option`
  let mut reader = backend.reader(&version_file)?;

  // Skip the 'start' amount of bytes without seek
  if let Some(skip) = start {
    reader.skip(skip.into());
    // io::copy(&mut reader.by_ref().take(skip.into()), &mut io::sink()).unwrap();
  }

  let async_reader = if let Some(limit) = end {
    let amount = limit - start.or(Some(0)).unwrap();
    ReadToAsyncRead {
      inner: Box::new(reader.take(amount.into())),
    }
  } else {
    ReadToAsyncRead { inner: reader }
  };

  // Create a FramedRead stream with BytesCodec for chunking
  let stream = FramedRead::new(async_reader, BytesCodec::new())
    // Use StreamExt::map to transform each Result item
    .map(|result_item| {
      result_item
        // Apply Result::map to transform Ok(BytesMut) to Ok(Vec<u8>)
        .map(|bytes| bytes.to_vec())
        // Apply Result::map_err to transform Err(std::io::Error) to Err(napi::Error)
        .map_err(|e| napi::Error::from(e)) // napi::Error implements From<tokio::io::Error>
    });
  // Create the napi-rs ReadableStream from the tokio_stream::Stream
  // The unwrap() here means if stream creation fails, it will panic.
  // For a production system, consider returning Result<Option<...>> and handling this.
  Some(ReadableStream::create_with_stream_bytes(env, stream).unwrap())
}

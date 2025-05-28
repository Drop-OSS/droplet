#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  fs::{self, metadata, File},
  io::{self, BufReader, ErrorKind, Read, Seek},
  path::{Path, PathBuf},
  task::Poll,
};

use napi::{
  bindgen_prelude::*,
  tokio_stream::{Stream, StreamExt},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, Take};
use tokio_util::{
  bytes::BytesMut,
  codec::{BytesCodec, FramedRead},
};

fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) {
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

pub struct VersionFile {
  pub relative_filename: String,
  pub permission: u32,
}

pub trait VersionBackend: 'static {
  fn list_files(&self, path: &Path) -> Vec<VersionFile>;
  fn reader(&self, file: &VersionFile) -> Option<File>;
}

pub struct PathVersionBackend {
  pub base_dir: PathBuf,
}
impl VersionBackend for PathVersionBackend {
  fn list_files(&self, path: &Path) -> Vec<VersionFile> {
    let mut vec = Vec::new();
    _list_files(&mut vec, path);

    let mut results = Vec::new();

    for pathbuf in vec.iter() {
      let file = File::open(pathbuf.clone()).unwrap();
      let relative = pathbuf.strip_prefix(path).unwrap();
      let metadata = file.try_clone().unwrap().metadata().unwrap();
      let permission_object = metadata.permissions();
      let permissions = {
        let perm: u32;
        #[cfg(target_family = "unix")]
        {
          perm = permission_object.mode();
        }
        #[cfg(not(target_family = "unix"))]
        {
          perm = 0
        }
        perm
      };

      results.push(VersionFile {
        relative_filename: relative.to_string_lossy().to_string(),
        permission: permissions,
      });
    }

    results
  }

  fn reader(&self, file: &VersionFile) -> Option<File> {
    let file = File::open(self.base_dir.join(file.relative_filename.clone())).ok()?;

    return Some(file);
  }
}

// Todo implementation for archives
// Split into a separate impl for each type of archive
pub struct ArchiveVersionBackend {}
impl VersionBackend for ArchiveVersionBackend {
  fn list_files(&self, path: &Path) -> Vec<VersionFile> {
    todo!()
  }

  fn reader(&self, file: &VersionFile) -> Option<File> {
    todo!()
  }
}

pub fn create_backend_for_path(path: &Path) -> Option<Box<(dyn VersionBackend)>> {
  let is_directory = path.is_dir();
  if is_directory {
    return Some(Box::new(PathVersionBackend {
      base_dir: path.to_path_buf(),
    }));
  };

  /*
    Insert checks for whatever backend you like
  */

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
  let backend = create_backend_for_path(path).unwrap();
  let files = backend.list_files(path);
  files.into_iter().map(|e| e.relative_filename).collect()
}

#[napi]
pub fn read_file(
  path: String,
  sub_path: String,
  env: &Env,
  start: Option<u32>,
  end: Option<u32>
) -> Option<ReadableStream<'static, BufferSlice<'static>>> {
  let path = Path::new(&path);
  let backend = create_backend_for_path(path).unwrap();
  let version_file = VersionFile {
    relative_filename: sub_path,
    permission: 0, // Shouldn't matter
  };
  // Use `?` operator for cleaner error propagation from `Option`
  let mut reader = backend.reader(&version_file)?;

  // Can't do this in tokio because it requires a .await, which we can't do here
  if let Some(start) = start {
    reader.seek(io::SeekFrom::Start(start as u64)).unwrap();
  }
  
  // Convert std::fs::File to tokio::fs::File for async operations
  let reader = tokio::fs::File::from_std(reader);

  
  let boxed_reader: Box<dyn AsyncRead + Send + Unpin> = match end {
    Some(end_val) => Box::new(reader.take(end_val as u64)),
    None => Box::new(reader),
  };

  

  // Create a FramedRead stream with BytesCodec for chunking

  let stream = FramedRead::new(boxed_reader, BytesCodec::new())
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

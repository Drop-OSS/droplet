use std::{fmt::Debug, io::Read};

use dyn_clone::DynClone;
use tokio::io::{self, AsyncRead};

#[derive(Debug, Clone)]
pub struct VersionFile {
  pub relative_filename: String,
  pub permission: u32,
  pub size: u64,
}

pub trait MinimumFileObject: Read + Send {}
impl<T: Read + Send> MinimumFileObject for T {}

// Intentionally not a generic, because of types in read_file
pub struct ReadToAsyncRead<'a> {
  pub inner: Box<dyn Read + Send + 'a>,
}

const ASYNC_READ_BUFFER_SIZE: usize = 8128;

impl<'a> AsyncRead for ReadToAsyncRead<'a> {
  fn poll_read(
    mut self: std::pin::Pin<&mut Self>,
    _cx: &mut std::task::Context<'_>,
    buf: &mut tokio::io::ReadBuf<'_>,
  ) -> std::task::Poll<io::Result<()>> {
    let mut read_buf = [0u8; ASYNC_READ_BUFFER_SIZE];
    let read_size = ASYNC_READ_BUFFER_SIZE.min(buf.remaining());
    match self.inner.read(&mut read_buf[0..read_size]) {
      Ok(read) => {
        buf.put_slice(&read_buf[0..read]);
        std::task::Poll::Ready(Ok(()))
      }
      Err(err) => {
        std::task::Poll::Ready(Err(err))
      },
    }
  }
}

pub trait VersionBackend: DynClone {
  fn require_whole_files(&self) -> bool;
  fn list_files(&mut self) -> anyhow::Result<Vec<VersionFile>>;
  fn peek_file(&mut self, sub_path: String) -> anyhow::Result<VersionFile>;
  fn reader(
    &mut self,
    file: &VersionFile,
    start: u64,
    end: u64,
  ) -> anyhow::Result<Box<dyn MinimumFileObject + '_>>;
}

dyn_clone::clone_trait_object!(VersionBackend);

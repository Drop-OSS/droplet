use std::{
  fmt::Debug, io::{Read, Seek, SeekFrom}
};

use tokio::io::{self, AsyncRead};

#[derive(Debug, Clone)]
pub struct VersionFile {
  pub relative_filename: String,
  pub permission: u32,
  pub size: u64,
}

pub trait Skippable {
  fn skip(&mut self, amount: u64);
}
impl<T> Skippable for T
where
  T: Seek,
{
  fn skip(&mut self, amount: u64) {
    self.seek(SeekFrom::Start(amount)).unwrap();
  }
}

pub trait MinimumFileObject: Read + Send + Skippable {}
impl<T: Read + Send + Seek> MinimumFileObject for T {}

// Intentionally not a generic, because of types in read_file
pub struct ReadToAsyncRead {
  pub inner: Box<(dyn Read + Send)>,
  pub backend: Box<(dyn VersionBackend + Send)>,
}

impl AsyncRead for ReadToAsyncRead {
  fn poll_read(
    mut self: std::pin::Pin<&mut Self>,
    _cx: &mut std::task::Context<'_>,
    buf: &mut tokio::io::ReadBuf<'_>,
  ) -> std::task::Poll<io::Result<()>> {
    let mut read_buf = [0u8; 8192];
    let var_name = self.inner.read(&mut read_buf).unwrap();
    let amount = var_name;
    buf.put_slice(&read_buf[0..amount]);
    std::task::Poll::Ready(Ok(()))
  }
}

pub trait VersionBackend {
  fn list_files(&mut self) -> Vec<VersionFile>;
  fn reader(&mut self, file: &VersionFile) -> Option<Box<(dyn MinimumFileObject)>>;
}

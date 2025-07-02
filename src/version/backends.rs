use core::arch;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  fs::File,
  io::{self, Read},
  path::PathBuf,
  pin::Pin,
  rc::Rc,
  sync::Arc,
};

use rawzip::{
  FileReader, ReaderAt, ZipArchive, ZipArchiveEntryWayfinder, ZipEntry, RECOMMENDED_BUFFER_SIZE,
};

use crate::version::{
  types::{MinimumFileObject, Skippable, VersionBackend, VersionFile},
  utils::_list_files,
};

pub struct PathVersionBackend {
  pub base_dir: PathBuf,
}
impl VersionBackend for PathVersionBackend {
  fn list_files(&mut self) -> Vec<VersionFile> {
    let mut vec = Vec::new();
    _list_files(&mut vec, &self.base_dir);

    let mut results = Vec::new();

    for pathbuf in vec.iter() {
      let file = File::open(pathbuf.clone()).unwrap();
      let relative = pathbuf.strip_prefix(self.base_dir.clone()).unwrap();
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

  fn reader(&mut self, file: &VersionFile) -> Option<Box<(dyn MinimumFileObject + 'static)>> {
    let file = File::open(self.base_dir.join(file.relative_filename.clone())).ok()?;

    return Some(Box::new(file));
  }
}

pub struct ZipVersionBackend {
  archive: Arc<ZipArchive<FileReader>>,
}
impl ZipVersionBackend {
  pub fn new(archive: File) -> Self {
    let archive = ZipArchive::from_file(archive, &mut [0u8; RECOMMENDED_BUFFER_SIZE]).unwrap();
    Self {
      archive: Arc::new(archive),
    }
  }

  pub fn new_entry(&self, entry: ZipEntry<'_, FileReader>) -> ZipFileWrapper {
    ZipFileWrapper {
      archive: self.archive.clone(),
      wayfinder: entry.entry,
      offset: entry.body_offset,
      end_offset: entry.body_end_offset,
    }
  }
}
impl Drop for ZipVersionBackend {
  fn drop(&mut self) {
    println!("dropping archive");
  }
}

struct ZipFileWrapper {
  pub archive: Arc<ZipArchive<FileReader>>,
  wayfinder: ZipArchiveEntryWayfinder,
  offset: u64,
  end_offset: u64,
}

impl Read for ZipFileWrapper {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    let read_size = buf.len().min((self.end_offset - self.offset) as usize);
    let read = self
      .archive
      .reader
      .read_at(&mut buf[..read_size], self.offset)?;
    self.offset += read as u64;
    Ok(read)
  }
}
impl Skippable for ZipFileWrapper {
  fn skip(&mut self, amount: u64) {
    /*io::copy(
        &mut self.inner.reader().by_ref().take(amount),
        &mut io::sink(),
      )
      .unwrap();
    */
  }
}
impl MinimumFileObject for ZipFileWrapper {}

impl VersionBackend for ZipVersionBackend {
  fn list_files(&mut self) -> Vec<VersionFile> {
    let mut results = Vec::new();
    let read_buffer = &mut [0u8; RECOMMENDED_BUFFER_SIZE];
    let mut budget_iterator = self.archive.entries(read_buffer);
    while let Some(entry) = budget_iterator.next_entry().unwrap() {
      if entry.is_dir() {
        continue;
      }
      results.push(VersionFile {
        relative_filename: entry.file_safe_path().unwrap().to_string(),
        permission: 744, // apparently ZIPs with permissions are not supported by this library, so we let the owner do anything
      });
    }
    results
  }

  fn reader(&mut self, file: &VersionFile) -> Option<Box<(dyn MinimumFileObject)>> {
    let read_buffer = &mut [0u8; RECOMMENDED_BUFFER_SIZE];
    let mut entries = self.archive.entries(read_buffer);
    let entry = loop {
      if let Some(v) = entries.next_entry().unwrap() {
        if v.file_safe_path().unwrap().to_string() == file.relative_filename {
          break Some(v);
        }
      } else {
        break None;
      }
    }?;

    let wayfinder = entry.wayfinder();
    let local_entry = self.archive.get_entry(wayfinder).unwrap();

    let wrapper = self.new_entry(local_entry);

    Some(Box::new(wrapper))
  }
}

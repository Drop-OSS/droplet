#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  fs::File,
  io::{self, Read},
  path::PathBuf,
};
use zip::{read::ZipFile, ZipArchive};

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
  archive: ZipArchive<File>,
}
impl ZipVersionBackend {
  pub fn new(archive: PathBuf) -> Self {
    let handle = File::open(archive).unwrap();
    Self {
      archive: ZipArchive::new(handle).unwrap(),
    }
  }
}

struct ZipFileWrapper<'a> {
  inner: ZipFile<'a, File>,
}

impl Read for ZipFileWrapper<'_> {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    self.inner.read(buf)
  }
}
impl Skippable for ZipFileWrapper<'_> {
  fn skip(&mut self, amount: u64) {
    io::copy(&mut self.inner.by_ref().take(amount), &mut io::sink()).unwrap();
  }
}
impl MinimumFileObject for ZipFileWrapper<'_> {}

impl VersionBackend for ZipVersionBackend {
  fn list_files(&mut self) -> Vec<VersionFile> {
    let mut results = Vec::new();
    for i in 0..self.archive.len() {
      let entry = self.archive.by_index(i).unwrap();
      results.push(VersionFile {
        relative_filename: entry.name().to_owned(),
        permission: entry.unix_mode().or(Some(0)).unwrap(),
      });
    }
    results
  }

  fn reader(&mut self, file: &VersionFile) -> Option<Box<(dyn MinimumFileObject)>> {
    let file = self.archive.by_name(&file.relative_filename).ok()?;
    let zip_file_wrapper = ZipFileWrapper { inner: file };

    //Some(Box::new(zip_file_wrapper))
    None
  }
}

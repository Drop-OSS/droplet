#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  fs::{self, metadata, File},
  io::{self, Read, Sink},
  path::{Path, PathBuf},
  sync::Arc,
};

use flate2::read::DeflateDecoder;
use rawzip::{
  FileReader, ZipArchive, ZipArchiveEntryWayfinder, ZipEntry, ZipReader, RECOMMENDED_BUFFER_SIZE,
};

use crate::version::types::{MinimumFileObject, Skippable, VersionBackend, VersionFile};

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

#[derive(Clone)]
pub struct PathVersionBackend {
  pub base_dir: PathBuf,
}
impl VersionBackend for PathVersionBackend {
  fn list_files(&mut self) -> Vec<VersionFile> {
    let mut vec = Vec::new();
    _list_files(&mut vec, &self.base_dir);

    let mut results = Vec::new();

    for pathbuf in vec.iter() {
      let relative = pathbuf.strip_prefix(self.base_dir.clone()).unwrap();

      results.push(
        self
          .peek_file(relative.to_str().unwrap().to_owned())
          .unwrap(),
      );
    }

    results
  }

  fn reader(&mut self, file: &VersionFile) -> Option<Box<dyn MinimumFileObject + 'static>> {
    let file = File::open(self.base_dir.join(file.relative_filename.clone())).ok()?;

    return Some(Box::new(file));
  }

  fn peek_file(&mut self, sub_path: String) -> Option<VersionFile> {
    let pathbuf = self.base_dir.join(sub_path.clone());
    if !pathbuf.exists() {
      return None;
    };

    let file = File::open(pathbuf.clone()).unwrap();
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

    Some(VersionFile {
      relative_filename: sub_path,
      permission: permissions,
      size: metadata.len(),
    })
  }
}

#[derive(Clone)]
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

  pub fn new_entry<'archive>(
    &self,
    entry: ZipEntry<'archive, FileReader>,
  ) -> ZipFileWrapper<'archive> {
    let deflater = DeflateDecoder::new(entry.reader());
    ZipFileWrapper { reader: deflater }
  }
}

pub struct ZipFileWrapper<'archive> {
  reader: DeflateDecoder<ZipReader<'archive, FileReader>>,
}

impl<'a> Read for ZipFileWrapper<'a> {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    let read = self.reader.read(buf)?;
    Ok(read)
  }
}
impl<'a> Skippable for ZipFileWrapper<'a> {
  fn skip(&mut self, amount: u64) {
    io::copy(&mut self.take(amount), &mut Sink::default()).unwrap();
  }
}
impl<'a> MinimumFileObject for ZipFileWrapper<'a> {}

impl ZipVersionBackend {
  fn find_wayfinder(&mut self, filename: &str) -> Option<ZipArchiveEntryWayfinder> {
    let read_buffer = &mut [0u8; RECOMMENDED_BUFFER_SIZE];
    let mut entries = self.archive.entries(read_buffer);
    let entry = loop {
      if let Some(v) = entries.next_entry().unwrap() {
        if v.file_path().try_normalize().unwrap().as_ref() == filename {
          break Some(v);
        }
      } else {
        break None;
      }
    }?;

    let wayfinder = entry.wayfinder();

    Some(wayfinder)
  }
}
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
        relative_filename: String::from(entry.file_path().try_normalize().unwrap()),
        permission: entry.mode().permissions(),
        size: entry.uncompressed_size_hint(),
      });
    }
    results
  }

  fn reader(&mut self, file: &VersionFile) -> Option<Box<dyn MinimumFileObject + '_>> {
    let wayfinder = self.find_wayfinder(&file.relative_filename)?;
    let local_entry = self.archive.get_entry(wayfinder).unwrap();

    let wrapper = self.new_entry(local_entry);

    Some(Box::new(wrapper))
  }

  fn peek_file(&mut self, sub_path: String) -> Option<VersionFile> {
    let entry = self.find_wayfinder(&sub_path)?;

    Some(VersionFile {
      relative_filename: sub_path,
      permission: 0,
      size: entry.uncompressed_size_hint(),
    })
  }
}

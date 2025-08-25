#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  fs::{self, metadata, File},
  io::{self, Read, Seek, SeekFrom, Sink},
  path::{Path, PathBuf},
  sync::Arc,
};

use anyhow::anyhow;
use flate2::read::DeflateDecoder;
use rawzip::{
  CompressionMethod, FileReader, ZipArchive, ZipArchiveEntryWayfinder, ZipEntry, ZipVerifier,
  RECOMMENDED_BUFFER_SIZE,
};

use crate::version::types::{MinimumFileObject, VersionBackend, VersionFile};

pub fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) -> napi::Result<()> {
  if metadata(path)?.is_dir() {
    let paths = fs::read_dir(path)?;
    for path_result in paths {
      let full_path = path_result?.path();
      if metadata(&full_path)?.is_dir() {
        _list_files(vec, &full_path)?;
      } else {
        vec.push(full_path);
      }
    }
  };

  Ok(())
}

#[derive(Clone)]
pub struct PathVersionBackend {
  pub base_dir: PathBuf,
}
impl VersionBackend for PathVersionBackend {
  fn list_files(&mut self) -> anyhow::Result<Vec<VersionFile>> {
    let mut vec = Vec::new();
    _list_files(&mut vec, &self.base_dir)?;

    let mut results = Vec::new();

    for pathbuf in vec.iter() {
      let relative = pathbuf.strip_prefix(self.base_dir.clone())?;

      results.push(
        self.peek_file(
          relative
            .to_str()
            .ok_or(napi::Error::from_reason("Could not parse path"))?
            .to_owned(),
        )?,
      );
    }

    Ok(results)
  }

  fn reader(
    &mut self,
    file: &VersionFile,
    start: u64,
    end: u64,
  ) -> anyhow::Result<Box<dyn MinimumFileObject + 'static>> {
    let mut file = File::open(self.base_dir.join(file.relative_filename.clone()))?;

    if start != 0 {
      file.seek(SeekFrom::Start(start))?;
    }

    if end != 0 {
      return Ok(Box::new(file.take(end - start)));
    }

    Ok(Box::new(file))
  }

  fn peek_file(&mut self, sub_path: String) -> anyhow::Result<VersionFile> {
    let pathbuf = self.base_dir.join(sub_path.clone());
    if !pathbuf.exists() {
      return Err(anyhow!("Path doesn't exist."));
    };

    let file = File::open(pathbuf.clone())?;
    let metadata = file.try_clone()?.metadata()?;
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

    Ok(VersionFile {
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
  pub fn new(archive: File) -> anyhow::Result<Self> {
    let archive = ZipArchive::from_file(archive, &mut [0u8; RECOMMENDED_BUFFER_SIZE])?;
    Ok(Self {
      archive: Arc::new(archive),
    })
  }

  pub fn new_entry<'archive>(
    &self,
    entry: ZipEntry<'archive, FileReader>,
    compression_method: CompressionMethod,
    start: u64,
    end: u64,
  ) -> anyhow::Result<ZipFileWrapper<'archive>> {
    let deflater: Box<dyn Read + Send + 'archive> = match compression_method {
      CompressionMethod::Store => Box::new(entry.reader()),
      CompressionMethod::Deflate => Box::new(DeflateDecoder::new(entry.reader())),
      CompressionMethod::Deflate64 => Box::new(DeflateDecoder::new(entry.reader())),
      _ => Err(anyhow!(
        "unsupported decompression algorithm: {compression_method:?}"
      ))?,
    };

    let mut verifier = entry.verifying_reader(deflater);
    if start != 0 {
      io::copy(&mut (&mut verifier).take(start), &mut Sink::default())?;
    }

    Ok(ZipFileWrapper {
      reader: verifier,
      limit: (end - start) as usize,
      current: 0,
    })
  }
}

pub struct ZipFileWrapper<'archive> {
  reader: ZipVerifier<'archive, Box<dyn Read + Send + 'archive>, FileReader>,
  limit: usize,
  current: usize,
}

/**
 * This read implemention is a result of debugging hell
 * It should probably be replaced with a .take() call.
 */
impl<'a> Read for ZipFileWrapper<'a> {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    let has_limit = self.limit != 0;

    // End this stream if the read is the right size
    if has_limit && self.current >= self.limit {
      return Ok(0);
    }

    let read = self.reader.read(buf)?;
    if self.limit != 0 {
      self.current += read;
      if self.current > self.limit {
        let over = self.current - self.limit;
        return Ok(read - over);
      }
    }
    Ok(read)
  }
}
//impl<'a> MinimumFileObject for ZipFileWrapper<'a> {}

impl ZipVersionBackend {
  fn find_wayfinder(
    &mut self,
    filename: &str,
  ) -> anyhow::Result<(ZipArchiveEntryWayfinder, CompressionMethod)> {
    let read_buffer = &mut [0u8; RECOMMENDED_BUFFER_SIZE];
    let mut entries = self.archive.entries(read_buffer);
    let entry = loop {
      if let Some(v) = entries.next_entry()? {
        if v.file_path().try_normalize()?.as_ref() == filename {
          break Ok(v);
        }
      } else {
        break Err(anyhow!("failed to fetch zip file header."));
      }
    }?;

    let wayfinder = entry.wayfinder();

    Ok((wayfinder, entry.compression_method()))
  }
}
impl VersionBackend for ZipVersionBackend {
  fn list_files(&mut self) -> anyhow::Result<Vec<VersionFile>> {
    let mut results = Vec::new();
    let read_buffer = &mut [0u8; RECOMMENDED_BUFFER_SIZE];
    let mut budget_iterator = self.archive.entries(read_buffer);
    while let Some(entry) = budget_iterator.next_entry()? {
      if entry.is_dir() {
        continue;
      }
      results.push(VersionFile {
        relative_filename: String::from(entry.file_path().try_normalize()?),
        permission: entry.mode().permissions(),
        size: entry.uncompressed_size_hint(),
      });
    }
    Ok(results)
  }

  fn reader(
    &mut self,
    file: &VersionFile,
    start: u64,
    end: u64,
  ) -> anyhow::Result<Box<dyn MinimumFileObject + '_>> {
    let (wayfinder, compression_method) = self.find_wayfinder(&file.relative_filename)?;
    let local_entry = self
      .archive
      .get_entry(wayfinder)?;

    let wrapper = self.new_entry(local_entry, compression_method, start, end)?;

    Ok(Box::new(wrapper) as Box<dyn MinimumFileObject>)
  }

  fn peek_file(&mut self, sub_path: String) -> anyhow::Result<VersionFile> {
    let (entry, _) = self.find_wayfinder(&sub_path)?;

    Ok(VersionFile {
      relative_filename: sub_path,
      permission: 0,
      size: entry.uncompressed_size_hint(),
    })
  }
}

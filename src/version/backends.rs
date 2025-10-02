#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  cell::LazyCell,
  fs::{self, metadata, File},
  io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Sink},
  path::{Path, PathBuf},
  process::{Child, ChildStdout, Command, Stdio},
  sync::{Arc, LazyLock},
};

use anyhow::anyhow;

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

  fn require_whole_files(&self) -> bool {
    false
  }
}

pub static SEVEN_ZIP_INSTALLED: LazyLock<bool> =
  LazyLock::new(|| Command::new("7z").output().is_ok());

#[derive(Clone)]
pub struct ZipVersionBackend {
  path: String,
}
impl ZipVersionBackend {
  pub fn new(path: PathBuf) -> anyhow::Result<Self> {
    Ok(Self {
      path: path.to_str().expect("invalid utf path").to_owned(),
    })
  }
}

pub struct ZipFileWrapper {
  command: Child,
  reader: BufReader<ChildStdout>
}

impl ZipFileWrapper {
  pub fn new(mut command: Child) -> Self {
    let stdout = command.stdout.take().expect("failed to access stdout of 7z");
    let reader = BufReader::new(stdout);
    ZipFileWrapper { command, reader }
  }
}

/**
 * This read implemention is a result of debugging hell
 * It should probably be replaced with a .take() call.
 */
impl Read for ZipFileWrapper {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    self.reader.read(buf)
  }
}

impl Drop for ZipFileWrapper {
    fn drop(&mut self) {
        self.command.wait().expect("failed to wait for 7z exit");
    }
}

impl VersionBackend for ZipVersionBackend {
  fn list_files(&mut self) -> anyhow::Result<Vec<VersionFile>> {
    let mut list_command = Command::new("7z");
    list_command.args(vec!["l", "-ba", &self.path]);
    let result = list_command.output()?;
    if !result.status.success() {
      return Err(anyhow!(
        "failed to list files: code {:?}",
        result.status.code()
      ));
    }
    let raw_result = String::from_utf8(result.stdout)?;
    let files = raw_result.split("\n").filter(|v| v.len() > 0).map(|v| v.split(" ").filter(|v| v.len() > 0));
    let mut results = Vec::new();

    for mut file in files {
      let (date, time, attrs, size, compress, name) = (
        file.next().unwrap(),
        file.next().unwrap(),
        file.next().unwrap(),
        file.next().unwrap(),
        file.next().unwrap(),
        file.next().unwrap(),
      );
      println!("got line: {} {} {} {} {} {}", date, time, attrs, size, compress, name);
      results.push(VersionFile {
        relative_filename: name.to_owned(),
        permission: 0,
        size: size.parse().unwrap(),
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
    let mut read_command = Command::new("7z");
    read_command.args(vec!["e", "-so", &self.path, &file.relative_filename]);
    let output = read_command.stdout(Stdio::piped()).spawn().expect("failed to spawn 7z");
    Ok(Box::new(ZipFileWrapper::new(output)))
  }

  fn peek_file(&mut self, sub_path: String) -> anyhow::Result<VersionFile> {
    let files = self.list_files()?;
    let file = files
      .iter()
      .find(|v| v.relative_filename == sub_path)
      .expect("file not found");

    Ok(file.clone())
  }

  fn require_whole_files(&self) -> bool {
    true
  }
}

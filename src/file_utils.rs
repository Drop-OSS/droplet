#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
  fs::{self, metadata, File},
  io::BufReader,
  path::{Path, PathBuf},
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
  fn reader(&self, file: &VersionFile) -> BufReader<File>;
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

  fn reader(&self, file: &VersionFile) -> BufReader<File> {
    let file = File::open(self.base_dir.join(file.relative_filename.clone())).unwrap();
    let reader = BufReader::with_capacity(4096, file);
    return reader;
  }
}

// Todo implementation for archives
// Split into a separate impl for each type of archive
pub struct ArchiveVersionBackend {}
impl VersionBackend for ArchiveVersionBackend {
  fn list_files(&self, path: &Path) -> Vec<VersionFile> {
    todo!()
  }

  fn reader(&self, file: &VersionFile) -> BufReader<File> {
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

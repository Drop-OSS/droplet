use std::{fs::{self, metadata}, path::{Path, PathBuf}};

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

pub fn list_files(path: &Path) -> Vec<PathBuf> {
  let mut vec = Vec::new();
  _list_files(&mut vec, path);
  vec
}

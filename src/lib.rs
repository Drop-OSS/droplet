#![deny(clippy::all)]

const CHUNK_SIZE: usize = 1024 * 1024 * 16;

use file_utils::list_files;
use manifest::{generate_manifest, Manifest, ManifestChunk, ManifestRecord};
use napi::Error;
use std::{
  collections::HashMap,
  fs::File,
  io::{BufRead, BufReader},
  path::Path,
  sync::{Arc, Mutex},
};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub mod file_utils;
pub mod manifest;

#[macro_use]
extern crate napi_derive;

fn compress(buffer: &[u8], output_path: &Path, chunk_id: Uuid) {
  let chunk_path = output_path.join(chunk_id.to_string() + ".bin");
  let chunk_file = File::create_new(chunk_path).unwrap();

  zstd::stream::copy_encode(buffer, chunk_file, 9).unwrap();
}

#[napi]
pub async fn repack(source: String, output: String) -> Result<(), Error> {
  let source_path = Path::new(&source);
  let output_path = Path::new(&output);

  let files = list_files(source_path);

  let num_of_threads: u64 = 8;

  let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(num_of_threads.try_into().unwrap())
    .build()
    .unwrap();

  let queue_size = Arc::new(Mutex::new(0));

  pool.scope(|scope| {
    let mut manifest = Manifest {
      record: HashMap::new(),
    };

    for file_path in files {
      let file = File::open(file_path.clone()).unwrap();
      let permissions = file.try_clone().unwrap().metadata().unwrap().permissions();
      let mut reader = BufReader::with_capacity(CHUNK_SIZE, file);
      let relative = file_path.strip_prefix(source_path).unwrap();

      let mut record = ManifestRecord {
        chunks: Vec::new(),
        permissions: 0,
      };
      #[cfg(unix)]
      {
        record.permissions = permissions.mode();
      }

      let mut chunk_index = 0;
      loop {
        let mut buffer: Vec<u8> = Vec::new();
        reader.fill_buf().unwrap().clone_into(&mut buffer);
        let length = buffer.len();

        if length == 0 {
          break;
        }

        {
          *queue_size.lock().unwrap() += 1;
        }

        let chunk_id: Uuid = Uuid::new_v4();

        let queue_size_handle = queue_size.clone();
        scope.spawn(move |_scope| {
          compress(&buffer, output_path, chunk_id);
          let mut num = queue_size_handle.lock().unwrap();
          *num -= 1;
        });

        reader.consume(length);

        let chunk_record = ManifestChunk {
          uuid: chunk_id.to_string(),
          index: chunk_index,
        };
        record.chunks.push(chunk_record);
        chunk_index += 1;

        loop {
          let num = queue_size.lock().unwrap();
          if *num < num_of_threads {
            break;
          }
        }
      }

      manifest
        .record
        .insert(relative.to_str().unwrap().to_string(), record);

      println!("Queued {}", file_path.to_str().unwrap());
    }
  
    let manifest_path = output_path.join("manifest.drop");
    generate_manifest(manifest, &manifest_path);
  });

  return Ok(());
}

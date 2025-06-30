use std::{
  fs as stdfs,
  io::{self as stdio, Read},
  path::Path,
};

use crate::blocking::unblock;

pub struct File(stdfs::File);

// Only including the operations that takes a long time. Others are not worth possibly starting a
// thread for.
impl File {
  pub async fn open<P: AsRef<Path> + Send + 'static>(
    path: P,
  ) -> stdio::Result<Self> {
    let file = unblock(move || stdfs::File::open(path)).await;
    file.map(File)
  }

  pub async fn create<P: AsRef<Path> + Send + 'static>(
    path: P,
  ) -> stdio::Result<Self> {
    let file = unblock(move || stdfs::File::create(path)).await;
    file.map(File)
  }

  pub async fn create_new<P: AsRef<Path> + Send + 'static>(
    path: P,
  ) -> stdio::Result<Self> {
    let file = unblock(move || stdfs::File::create_new(path)).await;
    file.map(File)
  }

  pub async fn read_to_string(
    &'static mut self,
    buf: &'static mut String,
  ) -> stdio::Result<usize> {
    unblock(move || self.0.read_to_string(buf)).await
  }

  pub async fn read_to_end(
    &'static mut self,
    buf: &'static mut Vec<u8>,
  ) -> stdio::Result<usize> {
    unblock(move || self.0.read_to_end(buf)).await
  }
}

impl AsRef<stdfs::File> for File {
  fn as_ref(&self) -> &stdfs::File {
    &self.0
  }
}

pub async fn read_to_string<P: AsRef<Path> + Send + 'static>(
  path: P,
) -> stdio::Result<String> {
  unblock(|| stdfs::read_to_string(path)).await
}

pub async fn write<
  P: AsRef<Path> + Send + 'static,
  C: AsRef<[u8]> + Send + 'static,
>(
  path: P,
  contents: C,
) -> stdio::Result<()> {
  unblock(|| stdfs::write(path, contents)).await
}

pub async fn read<P: AsRef<Path> + Send + 'static>(
  path: P,
) -> stdio::Result<Vec<u8>> {
  unblock(|| stdfs::read(path)).await
}

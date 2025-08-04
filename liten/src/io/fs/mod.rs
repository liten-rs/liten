use std::{
  fs::OpenOptions, io, os::fd::AsRawFd, path::Path, string::FromUtf8Error,
};

use thiserror::Error;

const CHUNK_SIZE: usize = 20; // 4KB

pub async fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
  let file = OpenOptions::new().read(true).open(path)?;

  let mut buffer = Vec::new();
  let mut chunks = Some(Vec::from([0; CHUNK_SIZE]));

  loop {
    let (vec_, bytes_read) =
      super::Driver::read(file.as_raw_fd(), chunks.unwrap(), -1).await?;

    if bytes_read == 0 {
      break; // End of file
    }
    buffer.extend_from_slice(&vec_[0..bytes_read as usize]);

    chunks = Some(vec_);
  }

  Ok(buffer)
}

#[derive(Debug, Error)]
pub enum ReadToStringError {
  #[error("io error {0}")]
  Io(io::Error),
  #[error("non-utf8 error {0}")]
  NonUtf8(FromUtf8Error),
}

impl From<io::Error> for ReadToStringError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<FromUtf8Error> for ReadToStringError {
  fn from(value: FromUtf8Error) -> Self {
    Self::NonUtf8(value)
  }
}

pub async fn read_to_string<P: AsRef<Path>>(
  path: P,
) -> Result<String, ReadToStringError> {
  let file_contents = read(path).await?;

  Ok(String::from_utf8(file_contents)?)
}

use std::{
  fs::OpenOptions, io, os::fd::AsRawFd, path::Path, string::FromUtf8Error,
};

use thiserror::Error;

use crate::io::BufResult;

const CHUNK_SIZE: usize = 4096; // 4KB

pub async fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
  let file = OpenOptions::new().read(true).open(path)?;

  let mut buffer = Vec::new();
  let mut chunks = Vec::from([0; CHUNK_SIZE]);
  let mut index = 0;

  loop {
    let (result, vector) = lio::read(file.as_raw_fd(), chunks, index).await;

    let bytes_read = result?;

    if bytes_read == 0 {
      break; // End of file
    }
    index += bytes_read as u64 + 1;
    buffer.extend_from_slice(&vector[0..bytes_read as usize]);

    chunks = vector;
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

pub async fn write<P: AsRef<Path>>(
  path: P,
  data: Vec<u8>,
) -> BufResult<(), Vec<u8>> {
  let file =
    match OpenOptions::new().create(true).write(true).truncate(true).open(path)
    {
      Ok(file) => file,
      Err(err) => return (Err(err), data),
    };

  let (result, vector) = lio::write(file.as_raw_fd(), data, 0).await;

  if let Err(err) = result {
    return (Err(err), vector);
  };

  (Ok(()), vector)
}

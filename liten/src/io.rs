// mod driver;
mod ext;

pub use ext::*;

use std::{future::Future, io};

pub mod fs;
pub mod net;

pub type BufResult<T, B> = (std::io::Result<T>, B);

pub trait AsyncRead {
  fn read(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = BufResult<usize, Vec<u8>>>;
}

pub trait AsyncWrite {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = BufResult<usize, Vec<u8>>>;
  fn flush(&mut self) -> impl Future<Output = io::Result<()>>;
}

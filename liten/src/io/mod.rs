mod write_ext;
pub use write_ext::*;

use std::{
  io,
  task::{Context, Poll},
};

pub trait AsyncRead {
  fn poll_read(
    &mut self,
    cx: &mut Context,
    buf: &mut [u8],
  ) -> Poll<io::Result<usize>>;
}

pub trait AsyncWrite {
  fn poll_write(
    &mut self,
    cx: &mut Context,
    buf: &[u8],
  ) -> Poll<io::Result<usize>>;

  fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>>;
}

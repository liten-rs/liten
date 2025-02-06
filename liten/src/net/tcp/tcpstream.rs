use crate::io_loop::IoRegistration;

use futures_io::{AsyncRead, AsyncWrite};
use mio::{net as mionet, Interest};
use std::{
  io::{self, Write},
  net as stdnet,
  pin::Pin,
  task::{Context, Poll},
};

pub struct TcpStream {
  inner: IoRegistration<mionet::TcpStream>,
  //read: bool,
  //write: bool,
}

impl TcpStream {
  pub(crate) fn new(tcp: mionet::TcpStream) -> Self {
    Self {
      inner: IoRegistration::new(tcp, Interest::READABLE | Interest::WRITABLE),
      //read: true,
      //write: true,
    }
  }
}

impl AsyncWrite for TcpStream {
  fn poll_write(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    Poll::Ready(self.inner.inner().write(buf))
  }

  fn poll_flush(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(self.inner.inner().flush())
  }
  fn poll_close(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(self.inner.inner().shutdown(stdnet::Shutdown::Write))
  }
}

impl AsyncRead for TcpStream {
  fn poll_read(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &mut [u8],
  ) -> Poll<io::Result<usize>> {
    use std::io::Read;
    //loop {
    match self.inner.inner().read(buf) {
      Ok(value) => {
        return Poll::Ready(Ok(value));
      }
      Err(err) => return Poll::Ready(Err(err)),
      //}
    }
  }
}

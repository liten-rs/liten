mod connect;
pub use connect::*;

use crate::{context, io_loop::IoRegistration};

use futures_io::{AsyncRead, AsyncWrite};
use mio::{net as mionet, Interest};
use std::{
  io::{self, ErrorKind, Read, Write},
  net::{self as stdnet, ToSocketAddrs},
  pin::Pin,
  task::{Context, Poll},
};

pub struct TcpStream {
  inner: mionet::TcpStream,
  registration: IoRegistration,
}

impl Drop for TcpStream {
  fn drop(&mut self) {
    // Ignore errors.
    let _ = self.registration.deregister(&mut self.inner);
  }
}

impl TcpStream {
  /// Create a new TCP stream and issue a non-blocking connect to the
  /// specified address.
  pub fn connect(addr: impl ToSocketAddrs) -> io::Result<Connect> {
    let addrs = addr.to_socket_addrs()?;
    for addr in addrs {
      let mut mio_stream = mionet::TcpStream::connect(addr)?;
      return Ok(Connect::inherit_stream(mio_stream));
    }

    Err(io::Error::new(io::ErrorKind::InvalidInput, "Address not valid"))
  }

  pub fn shutdown(&self, how: stdnet::Shutdown) -> io::Result<()> {
    self.inner.shutdown(how)
  }

  pub(crate) fn from_mio(mio: mionet::TcpStream) -> Self {
    Self::inherit_mio_stream(mio)
  }

  /// This function assumes the TcpStream input has been registered as an event.
  pub(crate) fn inherit_mio_stream(mut mio: mionet::TcpStream) -> TcpStream {
    let registration =
      IoRegistration::new(Interest::READABLE | Interest::WRITABLE);
    registration.register(&mut mio);
    TcpStream { inner: mio, registration }
  }
}

impl io::Write for TcpStream {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    loop {
      match self.inner.write(buf) {
        Ok(value) => return Ok(value),
        Err(err) if err.kind() == ErrorKind::WouldBlock => continue,
        Err(err) => return Err(err),
      }
    }
  }

  fn flush(&mut self) -> io::Result<()> {
    loop {
      match self.inner.flush() {
        Ok(value) => return Ok(value),
        Err(err) if err.kind() == ErrorKind::WouldBlock => continue,
        Err(err) => return Err(err),
      }
    }
  }
}

impl futures_io::AsyncRead for TcpStream {
  fn poll_read(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut [u8],
  ) -> Poll<io::Result<usize>> {
    match self.inner.read(buf) {
      Ok(value) => Poll::Ready(Ok(value)),
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
        self.registration.register_io_waker(cx);

        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

impl std::io::Read for TcpStream {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    loop {
      match self.inner.read(buf) {
        Ok(value) => return Ok(value),
        Err(err) if err.kind() == ErrorKind::WouldBlock => continue,
        Err(err) => return Err(err),
      }
    }
  }
}

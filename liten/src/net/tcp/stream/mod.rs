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

      let registration =
        IoRegistration::new(Interest::READABLE | Interest::WRITABLE);

      registration.register(&mut mio_stream)?;

      return Ok(Connect::inherit_stream_and_registration(
        mio_stream,
        registration,
      ));
    }

    Err(io::Error::new(io::ErrorKind::InvalidInput, "Address not valid"))
  }

  pub(crate) fn from_mio(mio: mionet::TcpStream) -> Self {
    let registration =
      IoRegistration::new(Interest::READABLE | Interest::WRITABLE);
    Self::inherit_mio_registration(mio, registration)
  }

  /// This function assumes the TcpStream input has been registered as an event.
  pub(crate) fn inherit_mio_registration(
    mio: mionet::TcpStream,
    registration: IoRegistration,
  ) -> TcpStream {
    TcpStream { inner: mio, registration }
  }
}

impl Write for TcpStream {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.inner.write(buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    self.inner.flush()
  }
}

impl AsyncWrite for TcpStream {
  fn poll_write(
    mut self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    match self.inner.write(buf) {
      Ok(value) => Poll::Ready(Ok(value)),
      Err(err) if err.kind() == ErrorKind::WouldBlock => {
        let _ =
          context::get_context().io().poll(self.registration.token(), _cx);
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }

  fn poll_flush(
    mut self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(self.flush())
  }
  fn poll_close(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(self.inner.shutdown(stdnet::Shutdown::Write))
  }
}

impl AsyncRead for TcpStream {
  fn poll_read(
    mut self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &mut [u8],
  ) -> Poll<io::Result<usize>> {
    match self.inner.read(buf) {
      Ok(value) => Poll::Ready(Ok(value)),
      Err(err) if err.kind() == ErrorKind::WouldBlock => {
        let _ =
          context::get_context().io().poll(self.registration.token(), _cx);
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

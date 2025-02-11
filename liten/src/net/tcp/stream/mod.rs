mod connect;
pub use connect::*;

use crate::events::EventRegistration;

use mio::{net as mionet, Interest};
use std::{
  io::{self, ErrorKind},
  net::{self as stdnet, ToSocketAddrs},
};

pub struct TcpStream {
  inner: mionet::TcpStream,
  registration: EventRegistration,
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
      let mio_stream = mionet::TcpStream::connect(addr)?;
      return Ok(Connect::inherit_stream(mio_stream));
    }

    Err(io::Error::new(io::ErrorKind::InvalidInput, "Address not valid"))
  }

  // Partially to maintain compatibility with std.
  pub fn shutdown(&mut self, how: stdnet::Shutdown) -> io::Result<()> {
    match how {
      stdnet::Shutdown::Read => self.shutdown_read(),
      stdnet::Shutdown::Write => self.shutdown_write(),
      stdnet::Shutdown::Both => {
        self.shutdown_read()?;
        self.shutdown_write()?;

        Ok(())
      }
    }
  }

  pub fn shutdown_read(&mut self) -> io::Result<()> {
    if self.registration.is_write() {
      self.registration.reregister(&mut self.inner, Interest::WRITABLE)?;
    } else {
      self.registration.deregister(&mut self.inner)?;
    };

    self.inner.shutdown(stdnet::Shutdown::Read)
  }

  pub fn shutdown_write(&mut self) -> io::Result<()> {
    if self.registration.is_read() {
      self.registration.reregister(&mut self.inner, Interest::READABLE)?;
    } else {
      self.registration.deregister(&mut self.inner)?;
    };
    self.inner.shutdown(stdnet::Shutdown::Write)
  }

  /// This function assumes the TcpStream input has been registered as an event.
  pub(crate) fn inherit_mio_stream(mut mio: mionet::TcpStream) -> TcpStream {
    let registration =
      EventRegistration::new(Interest::READABLE | Interest::WRITABLE);
    registration.register(&mut mio).expect("Couldn't register TcpStream");
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

impl io::Read for TcpStream {
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

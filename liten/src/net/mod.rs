use std::{
  future::{self, Future},
  io::{self, Read, Write},
  net::{self as stdnet, SocketAddr, ToSocketAddrs},
  pin::Pin,
  task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};
use mio::{net as mionet, Interest, Token};

use crate::{
  context,
  io_loop::{IOEventLoop, IoRegistration},
};

pub struct TcpListener {
  inner: IoRegistration<mionet::TcpListener>,
}

impl TcpListener {
  pub fn bind<A>(addr: A) -> io::Result<TcpListener>
  where
    A: ToSocketAddrs,
  {
    let tcp = stdnet::TcpListener::bind(addr)?;
    tcp.set_nonblocking(true)?;

    let mut tcp = mionet::TcpListener::from_std(tcp);

    let io_registration = IoRegistration::new(tcp, Interest::READABLE);

    Ok(TcpListener { inner: io_registration })
  }

  pub fn accept(&self) -> Accept<'_> {
    Accept {
      token: self.inner.token(),
      listener: &self.inner.inner(),
      io_event: IOEventLoop::get(),
    }
  }

  pub fn inner(&self) -> &IoRegistration<mionet::TcpListener> {
    &self.inner
  }
}

impl futures_core::Stream for TcpListener {
  type Item = (TcpStream, SocketAddr);

  fn poll_next(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    match self.inner.inner().accept() {
      Ok((stream, addr)) => Poll::Ready(Some((TcpStream::new(stream), addr))),
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
        IOEventLoop::get().poll(self.inner.token(), cx);
        Poll::Pending
      }
      Err(err) => Poll::Ready(None),
    }
  }
}

pub struct Accept<'a> {
  listener: &'a mionet::TcpListener,
  token: Token,
  io_event: &'a IOEventLoop,
}

impl Future for Accept<'_> {
  type Output = io::Result<(TcpStream, SocketAddr)>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.listener.accept() {
      Ok((stream, addr)) => Poll::Ready(Ok((TcpStream::new(stream), addr))),
      Err(kind) if kind.kind() == io::ErrorKind::WouldBlock => {
        dbg!(self.io_event.poll(self.token, cx));
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

impl Drop for TcpListener {
  fn drop(&mut self) {
    IOEventLoop::get().deregister(&mut self.inner);
  }
}

pub struct TcpStream {
  inner: IoRegistration<mionet::TcpStream>,
  read: bool,
  write: bool,
}

impl TcpStream {
  fn new(tcp: mionet::TcpStream) -> Self {
    Self {
      inner: IoRegistration::new(tcp, Interest::READABLE | Interest::WRITABLE),
      read: true,
      write: true,
    }
  }
}

impl AsyncRead for TcpStream {
  fn poll_read(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut [u8],
  ) -> Poll<io::Result<usize>> {
    loop {
      if std::mem::replace(&mut self.read, false) {
        match self.inner.inner().read(buf) {
          Ok(value) => {
            self.read = true;
            return Poll::Ready(Ok(value));
          }
          Err(err) => if err.kind() == io::ErrorKind::WouldBlock {},
          Err(e) => return Poll::Ready(Err(e)),
        }
      }
    }
  }
}

impl AsyncWrite for TcpStream {
  fn poll_write(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    self.inner.inner().write(buf);
    todo!();
  }

  fn poll_flush(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(self.inner.inner().flush())
  }
  fn poll_close(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(self.inner.inner().shutdown(stdnet::Shutdown::Write))
  }
}

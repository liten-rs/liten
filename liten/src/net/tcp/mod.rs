mod tcpstream;
pub use tcpstream::*;

use std::{
  future::Future,
  io::{self},
  net::{self as stdnet, SocketAddr, ToSocketAddrs},
  os::fd::AsFd,
  pin::Pin,
  task::{Context, Poll},
};

use mio::{event::Source, net as mionet, unix::SourceFd, Interest, Token};

use crate::io_loop::{IOEventLoop, IoRegistration};

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

    let tcp = mionet::TcpListener::from_std(tcp);

    let tcp = IoRegistration::new(tcp, Interest::READABLE);
    Ok(TcpListener { inner: tcp })
  }

  pub fn accept(&self) -> Accept<'_> {
    Accept { inner: &self.inner }

    //self.inner.inner().accept()
    //use std::os::fd::AsRawFd;
    //
    //let inner = self.inner.inner();
    //
    //let raw_fd = inner.as_raw_fd();
    //Accept {
    //  inner: IoRegistration::new(
    //    SourceFd(&raw_fd),
    //    Interest::READABLE | Interest::WRITABLE,
    //  ),
    //}
  }

  //pub fn inner(&self) -> &IoRegistration<mionet::TcpListener> {
  //  &self.inner
  //}
}

pub struct Accept<'a> {
  inner: &'a IoRegistration<mionet::TcpListener>,
}

//impl<'a> Accept<'a> {
//  pub fn from_mio(listener: &mionet::TcpListener) -> Self {
//    use std::os::fd::AsRawFd;
//    let fd = listener.as_raw_fd();
//
//    let source_fd = SourceFd::from(fd);
//
//    Accept::from_fd(source_fd)
//
//    //let registration = IoRegistration::new(
//    //  listener.as_fd(),
//    //  Interest::READABLE | Interest::WRITABLE,
//    //);
//    //Self { inner: registration }
//  }
//}
//
//impl<'a> Accept<'a> {
//  pub fn from_fd(fd: SourceFd<'a>) -> Accept<'a> {
//    Accept {
//      inner: IoRegistration::new(fd, Interest::READABLE | Interest::WRITABLE),
//    }
//  }
//}

impl Future for Accept<'_> {
  type Output = io::Result<(TcpStream, SocketAddr)>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.inner.inner().accept() {
      Ok((stream, addr)) => Poll::Ready(Ok((TcpStream::new(stream), addr))),
      Err(kind) if kind.kind() == io::ErrorKind::WouldBlock => {
        // event has been removed to the IO loop. so it has to be reregistered
        if let Poll::Ready(Ok(())) =
          IOEventLoop::get().poll(self.inner.token(), cx)
        {
          let _ = IOEventLoop::get().poll(self.inner.token(), cx);
        };
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}
//impl futures_core::Stream for TcpListener {
//  type Item = (TcpStream, SocketAddr);
//
//  fn poll_next(
//    self: Pin<&mut Self>,
//    cx: &mut Context<'_>,
//  ) -> Poll<Option<Self::Item>> {
//    match self.inner.inner().accept() {
//      Ok((stream, addr)) => Poll::Ready(Some((TcpStream::new(stream), addr))),
//      Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
//        IOEventLoop::get().poll(self.inner.token(), cx);
//        Poll::Pending
//      }
//      Err(err) => Poll::Ready(None),
//    }
//  }
//}

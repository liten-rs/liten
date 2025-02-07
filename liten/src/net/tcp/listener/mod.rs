mod accept;
use std::{
  io,
  net::{SocketAddr, ToSocketAddrs},
  pin::Pin,
  task::{Context, Poll},
};

pub use accept::*;

use mio::{net as mionet, Interest};
use std::net as stdnet;

use crate::{context, io_loop::IoRegistration};

use super::TcpStream;

pub struct TcpListener {
  registration: IoRegistration,
  listener: mionet::TcpListener,
}

impl TcpListener {
  pub fn bind<A>(addr: A) -> io::Result<TcpListener>
  where
    A: ToSocketAddrs,
  {
    let tcp = stdnet::TcpListener::bind(addr)?;
    tcp.set_nonblocking(true)?;

    let mut listener = mionet::TcpListener::from_std(tcp);

    let registration = IoRegistration::new(Interest::READABLE);
    let _ = registration.register(&mut listener);
    Ok(TcpListener { registration, listener })
  }

  pub fn accept(&self) -> Accept<'_> {
    Accept::new(&self.listener, self.registration)
  }
}

impl Drop for TcpListener {
  fn drop(&mut self) {
    let _ = self.registration.deregister(&mut self.listener);
  }
}
impl futures_core::Stream for TcpListener {
  type Item = io::Result<(TcpStream, SocketAddr)>;

  fn poll_next(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    match self.listener.accept() {
      Ok((stream, addr)) => {
        Poll::Ready(Some(Ok((TcpStream::from_mio(stream), addr))))
      }
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
        let _ = context::get_context().io().poll(self.registration.token(), cx);
        Poll::Pending
      }
      Err(err) => Poll::Ready(Some(Err(err))),
    }
  }
}

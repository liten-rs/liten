use std::{
  future::Future,
  io,
  net::SocketAddr,
  pin::Pin,
  task::{Context, Poll},
};

use mio::net as mionet;

use crate::{context, io_loop::IoRegistration, net::TcpStream};

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Accept<'a> {
  inner: &'a mionet::TcpListener,
  registration: IoRegistration,
}

impl<'a> Accept<'a> {
  pub(crate) fn new(
    listener: &'a mionet::TcpListener,
    registration: IoRegistration,
  ) -> Accept<'a> {
    Self { inner: listener, registration }
  }
}

impl Future for Accept<'_> {
  type Output = io::Result<(TcpStream, SocketAddr)>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.inner.accept() {
      Ok((stream, addr)) => Poll::Ready(Ok((
        TcpStream::inherit_mio_registration(stream, self.registration),
        addr,
      ))),
      Err(kind) if kind.kind() == io::ErrorKind::WouldBlock => {
        let _ = context::get_context().io().poll(self.registration.token(), cx);
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

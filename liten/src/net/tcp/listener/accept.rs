use std::{
  future::Future,
  io,
  net::SocketAddr,
  pin::Pin,
  task::{Context, Poll},
};

use mio::{net as mionet, Interest};

use crate::{events::EventRegistration, net::TcpStream};

use super::TcpListener;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Accept<'a> {
  listener: &'a TcpListener,
  registration: EventRegistration,
  // We don't drop this after accepts lifetime because it's a reference and it's TcpListeners job
  // to drop this.
  // registration: &'a EventRegistration,
}

impl<'a> Accept<'a> {
  pub(crate) fn new(
    listener: &'a mut TcpListener, // listener: &'a mionet::TcpListener,
                                   // registration: &'a EventRegistration,
  ) -> Accept<'a> {
    // This is only readable because this IoRegistration is only used for listening for incoming
    // connections. TcpStream read and write operations are all blocking.
    //
    // This is because some trange bugs happen when trying for async io.
    let registration =
      EventRegistration::new(Interest::READABLE, &mut listener.0).unwrap();
    Self { listener, registration }
  }
}

impl Future for Accept<'_> {
  type Output = io::Result<(TcpStream, SocketAddr)>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.listener.0.accept() {
      Ok((stream, addr)) => {
        Poll::Ready(Ok((TcpStream::inherit_mio_stream(stream), addr)))
      }
      Err(kind) if kind.kind() == io::ErrorKind::WouldBlock => {
        self.registration.register_io_waker(cx);
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

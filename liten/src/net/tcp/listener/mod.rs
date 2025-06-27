mod accept;
use std::{
  future::Future,
  io,
  net::{SocketAddr, ToSocketAddrs},
  pin::Pin,
  task::{Context, Poll},
};

pub use accept::*;

use mio::{net as mionet, Interest};
use std::net as stdnet;

use crate::events::EventRegistration;

use super::TcpStream;

// This is just for passing down to the struct Accept.
pub struct TcpListener(mionet::TcpListener);

// impl Drop for TcpListener {
//   fn drop(&mut self) {
//     let _ = self.registration.deregister(&mut self.listener);
//   }
// }

impl TcpListener {
  pub fn bind<A>(addr: A) -> io::Result<TcpListener>
  where
    A: ToSocketAddrs,
  {
    let tcp = stdnet::TcpListener::bind(addr)?;
    tcp.set_nonblocking(true)?;

    let listener = mionet::TcpListener::from_std(tcp);

    // This is only readable because this IoRegistration is only used for listening for incoming
    // connections. TcpStream read and write operations are all blocking.
    //
    // This is because some trange bugs happen when trying for async io.
    // let registration = EventRegistration::new(Interest::READABLE);
    // let _ = registration.register(&mut listener);
    Ok(TcpListener(listener))
  }

  pub fn accept(&self) -> Accept<'_> {
    Accept::new(&self.listener, &self.registration)
  }
}

impl futures_core::Stream for TcpListener {
  type Item = io::Result<(TcpStream, SocketAddr)>;

  fn poll_next(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    let fut = std::pin::pin!(self.accept());
    match Future::poll(fut, cx) {
      Poll::Ready(value) => Poll::Ready(Some(value)),
      Poll::Pending => Poll::Pending,
    }
  }
}

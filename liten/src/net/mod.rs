use std::{
  future::Future,
  io,
  net::{SocketAddr, ToSocketAddrs},
};

use mio::{net::TcpStream, Interest, Token};

use crate::{context, reactor::Reactor};

pub struct TcpListener {
  inner: mio::net::TcpListener,
  token: Token,
}

impl TcpListener {
  pub fn bind<A>(addr: A) -> io::Result<Self>
  where
    A: ToSocketAddrs,
  {
    let tcp = std::net::TcpListener::bind(addr)?;
    tcp.set_nonblocking(true)?;

    let mut tcp = mio::net::TcpListener::from_std(tcp);
    let token = Token(context::get_context().task_id_inc());

    Reactor::get().register(
      &mut tcp,
      token,
      Interest::WRITABLE | Interest::READABLE,
    );
    Ok(TcpListener {
      inner: tcp,
      token: Token(context::get_context().task_id_inc()),
    })
  }

  pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
    loop {
      match dbg!(self.inner.accept()) {
        Ok(value) => {
          println!("nice");
          return Ok(value);
        }
        Err(err) => match err.kind() {
          io::ErrorKind::WouldBlock => {
            std::future::poll_fn(|cx| Reactor::get().poll(self.token, cx))
              .await?
          }
          _ => return Err(err),
        },
      }
    }
  }
}

pub struct TcpAcceptFuture<'a> {
  fut: &'a mio::net::TcpListener,
}

//impl Future for TcpAcceptFuture<'_> {
//  type Output = io::Result<(TcpStream, SocketAddr)>;
//  fn poll(
//    self: std::pin::Pin<&mut Self>,
//    _cx: &mut std::task::Context<'_>,
//  ) -> Poll<Self::Output> {
//    match self.fut.accept() {
//      Ok(value) => Poll::Ready(Ok(value)),
//      Err(err) => {
//        if err.kind() == io::ErrorKind::WouldBlock {
//          Poll::Pending
//        } else {
//          Poll::Ready(Err(err))
//        }
//      }
//    }
//  }
//}

impl Drop for TcpListener {
  fn drop(&mut self) {
    Reactor::get().deregister(&mut self.inner)
  }
}

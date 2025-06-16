use std::{
  future::Future,
  io,
  pin::Pin,
  task::{Context, Poll},
};

use mio::{net as mionet, Interest};

use crate::{context, events::EventRegistration};

use super::TcpStream;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Connect {
  socket: Option<mionet::TcpStream>,
  registration: EventRegistration,
}

impl Connect {
  /// Registration and it's management is passed on
  pub(crate) fn inherit_stream(mut stream: mionet::TcpStream) -> Self {
    let registration = EventRegistration::new(Interest::READABLE);
    registration.register(&mut stream).expect("internal 'liten' error: failed to register liten::net::tcp::stream::Connect's IoRegistration");
    Self { socket: Some(stream), registration }
  }
}

impl Drop for Connect {
  fn drop(&mut self) {
    match self.socket {
      Some(ref mut v) => {
        // Ignore error
        let _ = self.registration.deregister(v);
      }
      None => {} // Future was dropped without polling
    }
  }
}

impl Future for Connect {
  type Output = io::Result<TcpStream>;
  fn poll(
    mut self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    match self.socket.take() {
      Some(socket) => {
        if let Ok(Some(err)) | Err(err) = socket.take_error() {
          return Poll::Ready(Err(err));
        }

        match socket.peer_addr() {
          Ok(_) => {
            let stream = TcpStream::inherit_mio_stream(socket);
            Poll::Ready(Ok(stream))
          }
          Err(err)
            if err.kind() == io::ErrorKind::NotConnected
              || err.raw_os_error()
                == Some(115 /* = libc::EINPROGRESS */) =>
          {
            context::with_context(|ctx| {
              ctx.handle().io().poll(self.registration.token(), _cx)
            });
            self.socket = Some(socket);

            Poll::Pending
          }
          Err(err) => Poll::Ready(Err(err)),
        }
      }
      None => panic!("polled Connect after completion"),
    }
  }
}

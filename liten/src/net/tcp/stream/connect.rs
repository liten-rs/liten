use std::{
  future::Future,
  io,
  pin::Pin,
  task::{Context, Poll},
};

use mio::net as mionet;

use crate::{context, io_loop::IoRegistration};

use super::TcpStream;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Connect {
  socket: Option<mionet::TcpStream>,
  registration: IoRegistration,
}

impl Connect {
  /// Registration and it's management is passed on
  pub(crate) fn inherit_stream_and_registration(
    stream: mionet::TcpStream,
    registration: IoRegistration,
  ) -> Self {
    Self { socket: Some(stream), registration }
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
            let stream =
              TcpStream::inherit_mio_registration(socket, self.registration);
            Poll::Ready(Ok(stream))
          }
          Err(err)
            if err.kind() == io::ErrorKind::NotConnected
              || err.raw_os_error() == Some(libc::EINPROGRESS) =>
          {
            let _ =
              context::get_context().io().poll(self.registration.token(), _cx);
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

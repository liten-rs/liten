use std::future::poll_fn;
use std::pin::Pin;
use std::prelude::rust_2024::Future;
use std::task::Poll;

use tokio::io::AsyncRead as TokioRead;
use tokio::io::AsyncWrite as TokioWrite;
use tokio::io::ReadBuf;

use crate::io::AsyncRead;
use crate::io::AsyncWrite;

pub struct TokioIo<T> {
  m: T,
}

impl<T: TokioRead + Unpin> AsyncRead for TokioIo<T> {
  fn read(
    &mut self,
    mut buf: Vec<u8>,
  ) -> impl Future<Output = (std::io::Result<usize>, Vec<u8>)> {
    async {
      let mut pinned = Pin::new(&mut self.m);
      let result = poll_fn(|cx| {
        let mut readbuf = ReadBuf::new(&mut buf);
        match pinned.as_mut().poll_read(cx, &mut readbuf) {
          Poll::Ready(value) => {
            if let Err(err) = value {
              Poll::Ready(Err(err))
            } else {
              let bytes_read = readbuf.filled().len();
              Poll::Ready(Ok(bytes_read))
            }
          }
          Poll::Pending => Poll::Pending,
        }
      });

      (result.await, buf)
    }
  }
}

impl<T: TokioWrite + Unpin> AsyncWrite for TokioIo<T> {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = crate::io::BufResult<usize, Vec<u8>>> {
    async {
      let mut pinned = Pin::new(&mut self.m);

      let result = poll_fn(|cx| match pinned.as_mut().poll_write(cx, &buf) {
        Poll::Ready(value) => Poll::Ready(value),
        Poll::Pending => Poll::Pending,
      });

      (result.await, buf)
    }
  }
  fn flush(&mut self) -> impl Future<Output = std::io::Result<()>> {
    let mut pinned = Pin::new(&mut self.m);
    poll_fn(move |cx| pinned.as_mut().poll_flush(cx))
  }
}

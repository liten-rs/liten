use std::io;
use std::{
  future::Future,
  task::{ready, Context, Poll},
};

use crate::io::AsyncWrite;

pub struct WriteAll<'a, T: ?Sized> {
  writer: &'a mut T,
  buf: &'a [u8],
}

impl<'a, T: AsyncWrite + ?Sized> WriteAll<'a, T> {
  pub fn new(writer: &'a mut T, buf: &'a [u8]) -> Self {
    Self { writer, buf }
  }
}

impl<T: AsyncWrite> Future for WriteAll<'_, T> {
  type Output = io::Result<()>;
  fn poll(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let this = &mut *self;

    while !this.buf.is_empty() {
      let num = ready!(this.writer.poll_write(cx, this.buf))?;

      let (_, rest) = std::mem::take(&mut this.buf).split_at(num);

      this.buf = rest;

      if num == 0 {
        return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
      }
    }

    Poll::Ready(Ok(()))
  }
}

use std::io;
use std::{
  future::Future,
  task::{Context, Poll},
};

use crate::io::AsyncWrite;

pub struct Flush<'a, T: ?Sized>(&'a mut T);

impl<'a, T: AsyncWrite + ?Sized> Flush<'a, T> {
  pub fn new(writer: &'a mut T) -> Self {
    Self(writer)
  }
}

impl<T: AsyncWrite> Future for Flush<'_, T> {
  type Output = io::Result<()>;
  fn poll(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    self.0.poll_flush(cx)
  }
}

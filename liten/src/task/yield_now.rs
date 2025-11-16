use std::{
  future::Future,
  task::{Context, Poll},
};

#[derive(Default)]
pub struct YieldNow {
  is_ready: u8,
}

impl Future for YieldNow {
  type Output = ();

  fn poll(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    if self.is_ready == 3 {
      return Poll::Ready(());
    }

    self.is_ready += 1;

    cx.waker().wake_by_ref();

    Poll::Pending
  }
}

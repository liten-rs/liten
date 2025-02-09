mod private {
  use std::{
    future::Future,
    task::{Context, Poll},
  };

  #[derive(Default)]
  pub struct YieldNow {
    is_ready: bool,
  }

  impl Future for YieldNow {
    type Output = ();

    fn poll(
      mut self: std::pin::Pin<&mut Self>,
      cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
      if self.is_ready {
        return Poll::Ready(());
      }

      self.is_ready = true;

      cx.waker().wake_by_ref();

      Poll::Pending
    }
  }
}

pub async fn yield_now() {
  private::YieldNow::default().await
}

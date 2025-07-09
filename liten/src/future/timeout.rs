#[cfg(feature = "time")]
use std::{future::Future, pin::Pin, task};

/// Error returned when a future times out.
///
/// This error is produced by the [`FutureExt::timeout`] method when the inner future does not complete within the specified duration.
#[cfg(feature = "time")]
#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Timeout reached")]
pub struct Timeout;

#[cfg(feature = "time")]
pin_project_lite::pin_project! {
  pub struct FutureTimeout<F: Future, TF: Future> {
    #[pin]
    pub future: F,
    #[pin]
    pub sleep_future: TF,
  }
}

#[cfg(feature = "time")]
impl<F, TF> Future for FutureTimeout<F, TF>
where
  F: Future,
  TF: Future,
{
  type Output = Result<F::Output, Timeout>;
  fn poll(
    self: Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Self::Output> {
    use std::task;

    let mut this = self.project();

    match this.future.as_mut().poll(cx) {
      task::Poll::Pending => match this.sleep_future.as_mut().poll(cx) {
        task::Poll::Pending => task::Poll::Pending,
        task::Poll::Ready(_) => task::Poll::Ready(Err(Timeout)),
      },
      task::Poll::Ready(value) => task::Poll::Ready(Ok(value)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::testing_util::noop_waker;
  use std::future::{pending, ready};
  use std::pin::pin;
  use std::task::Context;
  use std::time::Duration;

  #[crate::internal_test]
  #[cfg(feature = "time")]
  fn future_timeout_completes_before_timeout() {
    // A future that is immediately ready
    let fut = ready(123);
    let sleep = crate::time::sleep(Duration::from_millis(100));
    let mut timeout = pin!(FutureTimeout { future: fut, sleep_future: sleep });
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let poll = timeout.as_mut().poll(&mut cx);
    assert!(matches!(poll, std::task::Poll::Ready(Ok(123))));
  }

  #[crate::internal_test]
  #[cfg(feature = "time")]
  fn future_timeout_with_async_await() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      // Test that a future completing before timeout works correctly
      let fut = ready(42);
      let timeout_fut = FutureTimeout {
        future: fut,
        sleep_future: crate::time::sleep(Duration::from_millis(100)),
      };
      let result = timeout_fut.await;
      assert_eq!(result, Ok(42));
    });
  }

  #[crate::internal_test]
  #[cfg(feature = "time")]
  fn future_timeout_with_async_await_times_out() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      // Test that a future timing out works correctly
      let fut = pending::<()>();
      let timeout_fut = FutureTimeout {
        future: fut,
        sleep_future: crate::time::sleep(Duration::from_millis(0)),
      };
      let result = timeout_fut.await;
      assert_eq!(result, Err(Timeout));
    });
  }
}

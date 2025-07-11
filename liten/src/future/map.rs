use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use crate::future::util::FnOnce1;

pin_project_lite::pin_project! {
  pub struct Map<F1, Fun> {
    #[pin]
    f1: F1,
    fun: Option<Fun>,
  }
}

impl<F1, Fun> Map<F1, Fun> {
  pub(crate) fn new(f1: F1, fun: Fun) -> Self {
    Map { f1, fun: Some(fun) }
  }
}

impl<Fut, Fun, R> Future for Map<Fut, Fun>
where
  Fut: Future,
  Fun: FnOnce1<Fut::Output, Output = R>,
{
  type Output = R;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut this = self.project();

    match this.f1.as_mut().poll(cx) {
      Poll::Ready(value) => Poll::Ready(
        this
          .fun
          .take()
          .expect("liten::future::Map polled after Poll::Ready")
          .call_once(value),
      ),
      Poll::Pending => Poll::Pending,
    }
  }
}

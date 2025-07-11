use std::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

pin_project_lite::pin_project! {
  pub struct Or<F1, F2, T> {
    #[pin]
    f1: F1,
    #[pin]
    f2: F2,

    _marker: PhantomData<T>,
  }
}

impl<F1, F2, T> Or<F1, F2, T> {
  pub(crate) fn new(f1: F1, f2: F2) -> Self {
    Or { f1, f2, _marker: PhantomData }
  }
}

impl<F1, F2, T> Future for Or<F1, F2, T>
where
  F1: Future<Output = T>,
  F2: Future<Output = T>,
{
  type Output = T;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut this = self.project();

    if let Poll::Ready(value) = this.f1.as_mut().poll(cx) {
      return Poll::Ready(value);
    };

    if let Poll::Ready(value) = this.f2.as_mut().poll(cx) {
      return Poll::Ready(value);
    };

    Poll::Pending
  }
}

use std::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use crate::{CheckRegistrationResult, Driver, op};

type OperationId = u64;

pub struct OperationProgress<T> {
  id: OperationId,
  _m: PhantomData<T>,
}

impl<T> OperationProgress<T> {
  pub fn new(id: u64) -> Self {
    Self { id, _m: PhantomData }
  }
  /// Doesn't ty this progress down to any object inwhich lifetime is active.
  pub fn detatch(self) {
    // Engages the Driver::detatch(..);
    drop(self);
  }
}

impl<T> Future for OperationProgress<T>
where
  T: op::Operation,
{
  type Output = T::Result;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let is_done = Driver::get()
      .check_registration::<T>(self.id, cx.waker().clone())
      .expect("Polled OperationProgress when not even registered");

    match is_done {
      CheckRegistrationResult::WakerSet => Poll::Pending,
      CheckRegistrationResult::Value(result) => Poll::Ready(result),
    }
  }
}

impl<T> Drop for OperationProgress<T> {
  fn drop(&mut self) {
    Driver::get().detatch(self.id);
  }
}

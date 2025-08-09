use std::{
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use crate::{CheckRegistrationResult, Driver, op};

type OperationId = u64;

pub enum OperationProgress<T> {
  Async { id: OperationId, _m: PhantomData<T> },
  Sync { operation: T },
}

impl<T> OperationProgress<T> {
  pub fn new_async(id: u64) -> Self {
    Self::Async { id, _m: PhantomData }
  }

  pub fn new_sync(op: T) -> Self {
    Self::Sync { operation: op }
  }
  /// Doesn't ty this progress down to any object inwhich lifetime is active.
  pub fn detatch(self) {
    // Engages the Driver::detatch(..);
    drop(self);
  }
}

impl<T> Future for OperationProgress<T>
where
  T: op::Operation + Unpin,
{
  type Output = T::Result;

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    match *self {
      OperationProgress::Async { ref id, ref _m } => {
        let is_done = Driver::get()
          .check_registration::<T>(*id, cx.waker().clone())
          .expect("Polled OperationProgress when not even registered");

        match is_done {
          CheckRegistrationResult::WakerSet => Poll::Pending,
          CheckRegistrationResult::Value(result) => Poll::Ready(result),
        }
      }
      OperationProgress::Sync { ref mut operation } => {
        let result = operation.run_blocking();
        Poll::Ready(operation.result(result))
      }
    }
  }
}

impl<T> Drop for OperationProgress<T> {
  fn drop(&mut self) {
    if let OperationProgress::Async { id, _m } = *self {
      Driver::get().detatch(id);
    }
  }
}

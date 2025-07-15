use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll, Waker},
};

use crossbeam_queue::SegQueue;

use crate::loom::sync::atomic::{AtomicUsize, Ordering};

pub struct Semaphore {
  limit: AtomicUsize,
  waiters: SegQueue<Waker>,
}

impl Semaphore {
  pub fn new(size: usize) -> Self {
    assert!(size != 0, "Semaphore::new: 'size' cannot be 0.");

    Self { limit: AtomicUsize::new(size), waiters: SegQueue::new() }
  }

  fn inner_try_acquire(&self) -> bool {
    let mut count = self.limit.load(Ordering::Acquire);

    loop {
      if count == 0 {
        return false;
      }

      match self.limit.compare_exchange_weak(
        count,
        count - 1,
        Ordering::AcqRel,
        Ordering::Acquire,
      ) {
        Ok(_) => return true,
        Err(new_count) => count = new_count,
      }
    }
  }

  pub fn try_acquire(&self) -> Option<AcquireLock<'_>> {
    if self.inner_try_acquire() {
      Some(AcquireLock::new(self))
    } else {
      None
    }
  }

  pub fn acquire(&self) -> AcquireFuture<'_> {
    AcquireFuture { semaphore: self }
  }
}

pub struct AcquireFuture<'a> {
  semaphore: &'a Semaphore,
}

impl<'a> Future for AcquireFuture<'a> {
  type Output = AcquireLock<'a>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    if self.semaphore.inner_try_acquire() {
      Poll::Ready(AcquireLock::new(self.semaphore))
    } else {
      self.semaphore.waiters.push(cx.waker().clone());

      Poll::Pending
    }
  }
}

pub struct AcquireLock<'a> {
  semaphore: &'a Semaphore,
  // waker_id: Option<usize>,
}

impl<'a> AcquireLock<'a> {
  fn new(semaphore: &'a Semaphore) -> Self {
    Self { semaphore }
  }
  // fn with_waker(mut self, waker: usize) -> Self {
  //   self.waker_id = Some(waker);
  //   self
  // }
  pub fn release(self) {
    drop(self);
  }
}

impl Drop for AcquireLock<'_> {
  fn drop(&mut self) {
    let semaphore = self.semaphore;
    // upper limit does not need to be checked here. AcquireLock issuer is doing that
    semaphore.limit.fetch_add(1, Ordering::AcqRel);

    // If a future was used.
    while let Some(waker) = self.semaphore.waiters.pop() {
      waker.wake();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

  fn dummy_waker() -> Waker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
      dummy_raw_waker()
    }
    static VTABLE: RawWakerVTable =
      RawWakerVTable::new(clone, no_op, no_op, no_op);
    fn dummy_raw_waker() -> RawWaker {
      RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe { Waker::from_raw(dummy_raw_waker()) }
  }

  #[crate::internal_test]
  fn basic_acquire_release() {
    let s = Semaphore::new(1);
    let lock = s.try_acquire().unwrap();
    drop(lock);
    assert!(s.try_acquire().is_some());
  }

  #[crate::internal_test]
  fn try_acquire_fail() {
    let s = Semaphore::new(1);
    let _lock = s.try_acquire().unwrap();
    assert!(s.try_acquire().is_none());
  }

  #[crate::internal_test]
  fn waker_wakeup_on_release() {
    let s = Arc::new(Semaphore::new(1));
    let _lock = s.try_acquire().unwrap();
    let s2 = s.clone();
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    let mut fut = s2.acquire();
    assert!(matches!(Pin::new(&mut fut).poll(&mut cx), Poll::Pending));
    drop(_lock);
    // After dropping, should be acquirable
    assert!(s.try_acquire().is_some());
  }
}

use std::{
  collections::VecDeque,
  fmt::Display,
  future::Future,
  num::NonZero,
  pin::Pin,
  task::{Context, Poll, Waker},
};

use crate::loom::sync::{
  atomic::{AtomicUsize, Ordering},
  Mutex as StdMutex,
};

pub struct Semaphore {
  count: AtomicUsize,
  // This is not a bottleneck
  waiters: StdMutex<VecDeque<Waker>>,
}

impl Semaphore {
  pub fn with_size(size: NonZero<usize>) -> Self {
    Self {
      count: AtomicUsize::new(size.into()),
      waiters: StdMutex::new(VecDeque::new()),
    }
  }

  pub fn try_acquire(&self) -> Result<AcquireLock<'_>, AcquireLockError> {
    let count = self.count.load(Ordering::Acquire);

    if count > 0 {
      self.count.store(count - 1, Ordering::Release);
      Ok(AcquireLock(self))
    } else {
      Err(AcquireLockError)
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
    match self.semaphore.try_acquire() {
      Ok(lock) => Poll::Ready(lock),
      Err(_) => {
        let mut lock = self.semaphore.waiters.lock().unwrap();
        lock.push_back(cx.waker().clone());
        Poll::Pending
      }
    }
  }
}

pub struct AcquireLock<'a>(&'a Semaphore);

impl AcquireLock<'_> {
  pub fn release(self) {
    drop(self);
  }
}

impl Drop for AcquireLock<'_> {
  fn drop(&mut self) {
    let semaphore = self.0;
    let count = semaphore.count.fetch_add(1, Ordering::Release);

    if count == 0 {
      let mut lock = self.0.waiters.lock().unwrap();
      if let Some(waker) = lock.pop_front() {
        waker.wake();
      }
    }
  }
}

#[derive(Debug)]
pub struct AcquireLockError;

impl Display for AcquireLockError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "AcquireLockError: Failed to acquire lock")
  }
}

impl std::error::Error for AcquireLockError {}

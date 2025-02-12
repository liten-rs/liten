use std::{
  collections::VecDeque,
  fmt::Display,
  future::Future,
  num::NonZero,
  pin::Pin,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex as StdMutex,
  },
  task::{Context, Poll, Waker},
};

pub struct Semaphore {
  count: AtomicUsize,
  // This will not block basically anything.
  waiters: StdMutex<VecDeque<Waker>>,
}

impl Semaphore {
  pub fn with_size(size: NonZero<usize>) -> Self {
    Self {
      count: AtomicUsize::new(size.into()),
      waiters: StdMutex::new(VecDeque::new()),
    }
  }

  pub fn try_acquire<'a>(
    &'a self,
  ) -> Result<AcquireLock<'a>, AcquireLockError> {
    let count = self.count.load(Ordering::Acquire);

    if count > 0 {
      self.count.store(count - 1, Ordering::Release);
      Ok(AcquireLock(self))
    } else {
      Err(AcquireLockError)
    }
  }

  pub fn acquire<'a>(&'a self) -> AcquireFuture<'a> {
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
  pub fn release(&self) {
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

impl Drop for AcquireLock<'_> {
  fn drop(&mut self) {
    self.release();
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

#[test]
fn size_one() {
  let semaphore = Semaphore::with_size(1.try_into().unwrap());

  let lock = semaphore.try_acquire();
  assert!(lock.is_ok());

  let lock2 = semaphore.try_acquire();
  assert!(lock2.is_err());

  drop(lock);

  let lock3 = semaphore.try_acquire();
  assert!(lock3.is_ok());
}

#[test]
fn size_not_one() {
  let semaphore = Semaphore::with_size(3.try_into().unwrap());

  let lock = semaphore.try_acquire();
  assert!(lock.is_ok());

  let lock2 = semaphore.try_acquire();
  assert!(lock2.is_ok());

  let lock3 = semaphore.try_acquire();
  assert!(lock3.is_ok());

  let lock4 = semaphore.try_acquire();
  assert!(lock4.is_err());

  drop(lock3);

  let lock5 = semaphore.try_acquire();
  assert!(lock5.is_ok());

  let lock6 = semaphore.try_acquire();
  assert!(lock6.is_err());
}

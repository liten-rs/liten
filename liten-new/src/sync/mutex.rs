use std::{
  cell::UnsafeCell,
  ops::{Deref, DerefMut},
  panic::{RefUnwindSafe, UnwindSafe},
  thread,
};

use crate::loom::sync::atomic::{AtomicBool, Ordering};

use super::semaphore;
use thiserror::Error;

pub struct Mutex<T> {
  inner: std::cell::UnsafeCell<T>,
  poisoned: AtomicBool,
  guard: semaphore::Semaphore,
}

// Safety: Mutex logic makes sure this is safe.
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> UnwindSafe for Mutex<T> {}
impl<T> RefUnwindSafe for Mutex<T> {}

impl<T> Mutex<T> {
  pub fn new(value: T) -> Self {
    Self {
      inner: UnsafeCell::new(value),
      guard: semaphore::Semaphore::with_size(1.try_into().unwrap()),
      poisoned: AtomicBool::new(false),
    }
  }

  pub fn poison(&self) {
    self.poisoned.store(true, Ordering::Relaxed);
  }

  pub async fn lock(&self) -> Result<MutexGuard<'_, T>, PoisonError> {
    if self.poisoned.load(Ordering::Relaxed) {
      return Err(PoisonError);
    }
    let guard = self.guard.acquire().await;
    Ok(MutexGuard(self, guard))
  }

  pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, TryLockError> {
    let guard =
      self.guard.try_acquire().map_err(|_| TryLockError::UnableToAcquireLock);
    guard.map(|guard| MutexGuard(self, guard))
  }
}

#[derive(Debug, Error)]
#[error("PoisonError")]
pub struct PoisonError;

#[derive(Error, Debug, PartialEq)]
pub enum TryLockError {
  #[error("Unable to acquire lock")]
  UnableToAcquireLock,
}

#[allow(dead_code)]
pub struct MutexGuard<'a, T>(&'a Mutex<T>, semaphore::AcquireLock<'a>);

impl<T> Deref for MutexGuard<'_, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &*self.0.inner.get() }
  }
}

impl<T> DerefMut for MutexGuard<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *self.0.inner.get() }
  }
}

impl<T> Drop for MutexGuard<'_, T> {
  fn drop(&mut self) {
    if thread::panicking() {
      self.0.poison();
    }
  }
}

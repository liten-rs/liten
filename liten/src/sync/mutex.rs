//! Asynchronous-aware Mutex.
use std::{
  cell::UnsafeCell,
  ops::{Deref, DerefMut},
  panic::{RefUnwindSafe, UnwindSafe},
};

use crate::loom::{
  sync::atomic::{AtomicBool, Ordering},
  thread,
};

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
      guard: semaphore::Semaphore::new(1),
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
      self.guard.try_acquire().ok_or(TryLockError::UnableToAcquireLock);
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;

  #[cfg(feature = "runtime")]
  #[crate::internal_test]
  async fn basic_lock_unlock() {
    let m = Mutex::new(5);
    let guard =
      crate::runtime::Runtime::single_threaded().block_on(m.lock()).unwrap();
    assert_eq!(*guard, 5);
  }

  #[crate::internal_test]
  fn try_lock_success() {
    let m = Mutex::new(10);
    let guard = m.try_lock().unwrap();
    assert_eq!(*guard, 10);
  }

  #[crate::internal_test]
  fn try_lock_fail() {
    let m = Mutex::new(20);
    let _guard = m.try_lock().unwrap();
    assert!(matches!(
      m.try_lock(),
      Err(super::TryLockError::UnableToAcquireLock)
    ));
  }

  #[cfg(feature = "runtime")]
  #[crate::internal_test]
  fn poisoning_on_panic() {
    let m = Arc::new(Mutex::new(42));
    let m2 = m.clone();
    let _ = std::panic::catch_unwind(move || {
      let _guard =
        crate::runtime::Runtime::single_threaded().block_on(m2.lock()).unwrap();
      panic!("poison");
    });
    assert!(m.poisoned.load(std::sync::atomic::Ordering::Relaxed));
  }
}

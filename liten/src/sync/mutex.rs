use std::{
  cell::UnsafeCell,
  error::Error,
  fmt::Display,
  ops::{Deref, DerefMut},
  panic::{RefUnwindSafe, UnwindSafe},
  sync::atomic::AtomicBool,
};

use super::{AcquireLock, Semaphore};

pub struct Mutex<T> {
  inner: UnsafeCell<T>,
  poisoned: AtomicBool,
  guard: Semaphore,
}
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}
impl<T> UnwindSafe for Mutex<T> {}
impl<T> RefUnwindSafe for Mutex<T> {}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct AcquireLockError;

impl Error for AcquireLockError {}

impl Display for AcquireLockError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "AcquireLockError: Failed to acquire lock")
  }
}

pub struct PoisonError;

impl<T> Mutex<T> {
  pub fn new(value: T) -> Self {
    Self {
      inner: UnsafeCell::new(value),
      guard: Semaphore::with_size(1.try_into().unwrap()),
      poisoned: AtomicBool::new(false),
    }
  }

  pub fn poison(&self) {
    self.poisoned.store(true, std::sync::atomic::Ordering::Relaxed);
  }

  pub async fn lock(&self) -> Result<MutexGuard<'_, T>, PoisonError> {
    if self.poisoned.load(std::sync::atomic::Ordering::Relaxed) {
      return Err(PoisonError);
    }
    let guard = self.guard.acquire().await;
    Ok(MutexGuard(self, guard))
  }

  pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, AcquireLockError> {
    let guard = self.guard.try_acquire().map_err(|_| AcquireLockError);
    guard.map(|guard| MutexGuard(self, guard))
  }
}

pub struct MutexGuard<'a, T>(&'a Mutex<T>, AcquireLock<'a>);

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

impl<T> MutexGuard<'_, T> {
  pub fn release(self) {
    self.1.release();
  }
}

impl<T> Drop for MutexGuard<'_, T> {
  fn drop(&mut self) {
    if std::thread::panicking() {
      self.0.poison();
    }
    self.1.release();
  }
}

#[test]
fn lock() {
  let mutex = Mutex::new(0);

  let lock = mutex.try_lock();
  assert!(lock.is_ok());

  let mut value = lock.unwrap();

  *value += 1;
  assert_eq!(*value, 1);

  assert!(mutex.try_lock().is_err_and(|err| err == AcquireLockError));

  value.release();

  let value = mutex.try_lock();

  assert!(value.is_ok());

  let mut value = value.unwrap();

  assert!(*value == 1);

  *value += 1;
  assert!(*value == 2);
}

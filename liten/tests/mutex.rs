#![cfg(feature = "sync")]

use liten::sync::{Mutex, TryLockError};

#[liten::internal_test]
fn lock() {
  let mutex = Mutex::new(0);

  let lock = mutex.try_lock();
  let lock2 = mutex.try_lock();
  assert!(lock.is_ok());
  assert!(lock2.is_err());

  let mut value = lock.unwrap();

  *value += 1;
  assert_eq!(*value, 1);

  assert!(mutex
    .try_lock()
    .is_err_and(|err| err == TryLockError::UnableToAcquireLock));

  drop(value);

  let value = mutex.try_lock();

  assert!(value.is_ok());

  let mut value = value.unwrap();

  assert!(*value == 1);

  *value += 1;
  assert!(*value == 2);
}

use liten::testing_util::noop_waker;
use std::future::Future;
use std::task::Context;

use liten::sync::Semaphore;

macro_rules! get_ready {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!($expr);
    match pinned.as_mut().poll(&mut Context::from_waker(&noop_waker())) {
      std::task::Poll::Ready(value) => value,
      std::task::Poll::Pending => unreachable!("was Poll::Pending"),
    }
  }};
}

macro_rules! should_pending {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!(&mut $expr);
    match pinned.as_mut().poll(&mut Context::from_waker(&noop_waker())) {
      std::task::Poll::Ready(_) => false,
      std::task::Poll::Pending => true,
    }
  }};
}

#[liten::internal_test]
fn max_capacity() {
  let semaphore = Semaphore::new(2);

  let lock1 = get_ready!(semaphore.acquire());
  let lock2 = get_ready!(semaphore.acquire());

  let mut later_get = semaphore.acquire();

  should_pending!(later_get);

  lock1.release();

  get_ready!(later_get);

  lock2.release();
}

#[liten::internal_test]
fn size_one() {
  let semaphore = Semaphore::new(1);

  let lock = semaphore.try_acquire();
  assert!(lock.is_some());

  let lock2 = semaphore.try_acquire();
  assert!(lock2.is_none());

  drop(lock);

  let lock3 = semaphore.try_acquire();
  assert!(lock3.is_some());
}

#[liten::internal_test]
fn size_not_one() {
  let semaphore = Semaphore::new(3);

  let lock = semaphore.try_acquire();
  assert!(lock.is_some());

  let lock2 = semaphore.try_acquire();
  assert!(lock2.is_some());

  let lock3 = semaphore.try_acquire();
  assert!(lock3.is_some());

  let lock4 = semaphore.try_acquire();
  assert!(lock4.is_none());

  drop(lock3);

  let lock5 = semaphore.try_acquire();
  assert!(lock5.is_some());

  let lock6 = semaphore.try_acquire();
  assert!(lock6.is_none());
}

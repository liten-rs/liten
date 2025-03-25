#![cfg(loom)]

use std::future::Future;
use std::num::NonZero;
use std::task::Context;

use futures_task::noop_waker;
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

#[test]
fn max_capacity() {
  loom::model(|| {
    let semaphore = Semaphore::with_size(NonZero::new(2).unwrap());

    let lock1 = get_ready!(semaphore.acquire());
    let lock2 = get_ready!(semaphore.acquire());

    let mut later_get = semaphore.acquire();

    should_pending!(later_get);

    lock1.release();

    get_ready!(later_get);

    lock2.release();
  })
}

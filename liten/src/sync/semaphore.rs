use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll, Waker},
};

use dashmap::DashMap;

use crate::loom::sync::atomic::{AtomicUsize, Ordering};

struct WakersState {
  list: DashMap<usize, Waker>,
  next_lock_id: AtomicUsize,
}

impl WakersState {
  fn new() -> Self {
    Self { next_lock_id: AtomicUsize::new(0), list: DashMap::new() }
  }
  fn set_waker(&self, id: usize, waker: Waker) {
    self.list.insert(id, waker);
  }

  fn take_waker(&self, id: usize) -> Option<Waker> {
    self.list.remove(&id).map(|entry| entry.1)
  }
}

pub struct Semaphore {
  limit: AtomicUsize,
  waiters: WakersState,
}

impl Semaphore {
  pub fn new(size: usize) -> Self {
    assert!(size != 0, "Semaphore::new: 'size' cannot be 0.");

    Self { limit: AtomicUsize::new(size), waiters: WakersState::new() }
  }

  fn issue_next_waiter_id(&self) -> usize {
    self.waiters.next_lock_id.fetch_add(1, Ordering::AcqRel)
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
      Some(AcquireLock::new(&self))
    } else {
      None
    }
  }

  pub fn acquire(&self) -> AcquireFuture<'_> {
    AcquireFuture { semaphore: self, waiter_id: self.issue_next_waiter_id() }
  }
}

pub struct AcquireFuture<'a> {
  semaphore: &'a Semaphore,
  waiter_id: usize,
}

impl<'a> Future for AcquireFuture<'a> {
  type Output = AcquireLock<'a>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    if self.semaphore.inner_try_acquire() {
      Poll::Ready(AcquireLock::new(self.semaphore).with_waker(self.waiter_id))
    } else {
      self.semaphore.waiters.set_waker(self.waiter_id, cx.waker().clone());

      Poll::Pending
    }
  }
}

pub struct AcquireLock<'a> {
  semaphore: &'a Semaphore,
  waker_id: Option<usize>,
}

impl<'a> AcquireLock<'a> {
  fn new(semaphore: &'a Semaphore) -> Self {
    Self { waker_id: None, semaphore }
  }
  fn with_waker(mut self, waker: usize) -> Self {
    self.waker_id = Some(waker);
    self
  }
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
    if let Some(waiter_id) = self.waker_id {
      if let Some(waker) = self.semaphore.waiters.take_waker(waiter_id) {
        waker.wake();
      } else {
        // AcquireFuture didn't need to poll, so do nothing
      }
    }
  }
}

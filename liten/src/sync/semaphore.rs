use std::{
  cell::Cell,
  collections::HashMap,
  future::Future,
  pin::Pin,
  task::{Context, Poll, Waker},
};

use crate::loom::sync::{
  atomic::{AtomicUsize, Ordering},
  Mutex as StdMutex,
};

struct WakersState {
  list: HashMap<usize, Waker>,
  next_lock_id: Cell<usize>,
}

impl WakersState {
  fn new() -> Self {
    Self { next_lock_id: Cell::new(0), list: HashMap::new() }
  }
  fn set_waker(&mut self, id: usize, waker: Waker) {
    self.list.insert(id, waker);
  }

  fn take_waker(&mut self, id: usize) -> Option<Waker> {
    self.list.remove(&id)
  }
}

pub struct Semaphore {
  limit: AtomicUsize,
  // This is not a bottleneck
  waiters: StdMutex<WakersState>,
}

impl Semaphore {
  pub fn new(size: usize) -> Self {
    assert!(size != 0, "Semaphore::new: 'size' cannot be 0.");

    Self {
      limit: AtomicUsize::new(size),
      waiters: StdMutex::new(WakersState::new()),
    }
  }

  fn issue_next_waiter_id(&self) -> usize {
    let state = self.waiters.lock().unwrap();
    let this_waker_id = state.next_lock_id.get();
    state.next_lock_id.set(this_waker_id + 1);
    this_waker_id
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
      Some(AcquireLock(&self, None))
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
      Poll::Ready(AcquireLock(self.semaphore, Some(self.waiter_id)))
    } else {
      let mut lock = self.semaphore.waiters.lock().unwrap();
      lock.set_waker(self.waiter_id, cx.waker().clone());

      Poll::Pending
    }
  }
}

pub struct AcquireLock<'a>(&'a Semaphore, Option<usize>);

impl AcquireLock<'_> {
  pub fn release(self) {
    drop(self);
  }
}

impl Drop for AcquireLock<'_> {
  fn drop(&mut self) {
    // upper limit does not need to be checked here. AcquireLock issuer is doing that
    let semaphore = self.0;
    semaphore.limit.fetch_add(1, Ordering::AcqRel);

    // If a future was used.
    if let Some(waiter_id) = self.1 {
      if let Some(waker) = self.0.waiters.lock().unwrap().take_waker(waiter_id)
      {
        waker.wake();
      } else {
        // AcquireFuture didn't need to poll, so do nothing
      }
    }
  }
}

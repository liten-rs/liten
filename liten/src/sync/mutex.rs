use std::{
  cell::{RefCell, UnsafeCell},
  collections::VecDeque,
  future::Future,
  ops::{Deref, DerefMut},
  sync::atomic::{AtomicUsize, Ordering},
  task::{Context, Poll, Waker},
};

#[derive(Debug)]
pub struct Mutex<D: ?Sized> {
  current_ticket: AtomicUsize,
  /// Increments each time a task attempts to acquire the lock.
  ticket_counter: AtomicUsize,
  // Why Box? Right now a quickfix
  data: Box<UnsafeCell<D>>,
  wakers: RefCell<VecDeque<Waker>>,
}

unsafe impl<T> Send for Mutex<T> where T: Send {}
unsafe impl<T> Sync for Mutex<T> where T: Send {}
unsafe impl<T> Sync for MutexGuard<'_, T> where T: Send + Sync {}

pub struct MutexGuard<'a, D> {
  mutex: &'a Mutex<D>,
}
impl<'a, D> MutexGuard<'a, D> {
  fn new(mutex: &'a Mutex<D>) -> Self {
    MutexGuard { mutex }
  }
}

pub struct MutexGuardFuture<'a, D> {
  mutex: &'a Mutex<D>,
  ticket: usize,
}

impl<'a, D> MutexGuardFuture<'a, D> {
  fn new(mutex: &'a Mutex<D>) -> Self {
    let ticket = mutex.ticket_counter.fetch_add(1, Ordering::SeqCst);
    MutexGuardFuture { mutex, ticket }
  }
}

impl<D> Mutex<D> {
  pub fn new(value: D) -> Mutex<D> {
    Self {
      ticket_counter: AtomicUsize::new(0),
      current_ticket: AtomicUsize::new(0),
      data: Box::new(UnsafeCell::new(value)),
      wakers: RefCell::new(VecDeque::new()),
    }
  }
  pub fn lock<'a>(&'a self) -> MutexGuardFuture<'a, D> {
    MutexGuardFuture::new(self)
  }
}

impl<G> Deref for MutexGuard<'_, G> {
  type Target = G;
  fn deref(&self) -> &Self::Target {
    unsafe { self.mutex.data.get().as_ref() }.unwrap()
  }
}

impl<G> DerefMut for MutexGuard<'_, G> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.mutex.data.get().as_mut() }.unwrap()
  }
}

impl<S> Drop for MutexGuard<'_, S> {
  fn drop(&mut self) {
    self.mutex.ticket_counter.fetch_sub(1, Ordering::SeqCst);
    if let Some(waker) = self.mutex.wakers.borrow_mut().pop_front() {
      waker.wake();
    }
  }
}

impl<'a, D> Future for MutexGuardFuture<'a, D> {
  type Output = MutexGuard<'a, D>;
  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let this = self.get_mut();

    // Check if the current ticket matches the task's ticket
    if this.mutex.current_ticket.load(Ordering::SeqCst) == this.ticket {
      // If the ticket matches, the task acquires the lock
      Poll::Ready(MutexGuard::new(this.mutex))
    } else {
      // Register the waker and wait for the lock
      let mut wakers = this.mutex.wakers.borrow_mut();
      // Avoid registering waker if it's already there
      if !wakers.iter().any(|w| w.will_wake(cx.waker())) {
        wakers.push_back(cx.waker().clone());
      }
      Poll::Pending
    }
  }
}

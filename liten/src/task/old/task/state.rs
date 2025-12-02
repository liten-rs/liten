use std::sync::atomic::Ordering;
use std::task::Waker;
use std::{mem::MaybeUninit, ptr};

use std::{
  cell::UnsafeCell,
  sync::atomic::{AtomicPtr, AtomicU8},
};

use crate::task::TaskHandleError;

// State constants
const STATE_PENDING: u8 = 0;
const STATE_READY: u8 = 1;
const STATE_PANICKED: u8 = 2;
const STATE_CONSUMED: u8 = 3;

/// Sound task result state with minimal allocations
/// This uses Arc for sound lifetime management
pub struct TaskResultState<T> {
  // Atomic state: PENDING -> READY/PANICKED -> CONSUMED
  state: AtomicU8,
  // The actual result value (only valid when state is READY), doesn't need to be atomic because
  // legally, value is only set once.
  value: UnsafeCell<MaybeUninit<T>>,
  // Waker to notify the handle when the task completes
  waker: AtomicPtr<Waker>,
}

impl<T> TaskResultState<T> {
  pub fn new() -> Self {
    Self {
      state: AtomicU8::new(STATE_PENDING),
      value: UnsafeCell::new(MaybeUninit::uninit()),
      waker: AtomicPtr::new(ptr::null_mut()),
    }
  }

  pub fn set_ready(&self, value: T) {
    let result = self.state.compare_exchange(
      STATE_PENDING,
      STATE_READY,
      Ordering::AcqRel,
      Ordering::Acquire,
    );

    if result.is_ok() {
      unsafe {
        self.value.with_mut(|ptr_mut| (*ptr_mut).write(value));
        self.wake();
        self.clear_waker();
      }
    }
  }

  pub fn set_panicked(&self) {
    let result = self.state.compare_exchange(
      STATE_PENDING,
      STATE_PANICKED,
      Ordering::AcqRel,
      Ordering::Acquire,
    );

    if result.is_ok() {
      self.wake();
      self.clear_waker();
    }
  }

  pub fn try_take(&self) -> Option<Result<T, TaskHandleError>> {
    let current_state = self.state.load(Ordering::Acquire);

    match current_state {
      STATE_READY => {
        let result = self.state.compare_exchange(
          STATE_READY,
          STATE_CONSUMED,
          Ordering::AcqRel,
          Ordering::Acquire,
        );

        if result.is_ok() {
          let value =
            self.value.with(|ptr| unsafe { (*ptr).assume_init_read() });
          self.clear_waker();
          Some(Ok(value))
        } else {
          None
        }
      }
      STATE_PANICKED => {
        let result = self.state.compare_exchange(
          STATE_PANICKED,
          STATE_CONSUMED,
          Ordering::AcqRel,
          Ordering::Acquire,
        );

        if result.is_ok() {
          self.clear_waker();
          Some(Err(TaskHandleError::BodyPanicked))
        } else {
          None
        }
      }
      _ => None,
    }
  }

  pub fn set_waker(&self, waker: Waker) {
    let boxed = Box::new(waker);
    let ptr = Box::into_raw(boxed);
    let old = self.waker.swap(ptr, Ordering::AcqRel);
    if !old.is_null() {
      // Drop the old waker
      unsafe {
        drop(Box::from_raw(old));
      }
    }
  }

  pub fn wake(&self) {
    let ptr = self.waker.load(Ordering::Acquire);
    if !ptr.is_null() {
      // Wake by ref, but do not drop here
      unsafe {
        (*ptr).wake_by_ref();
      }
    }
  }

  pub fn clear_waker(&self) {
    let ptr = self.waker.swap(ptr::null_mut(), Ordering::AcqRel);
    if !ptr.is_null() {
      unsafe {
        drop(Box::from_raw(ptr));
      }
    }
  }
}

// SAFETY:
// - T: Send ensures the value can be safely shared between threads
// - AtomicU8 provides thread-safe state management
// - UnsafeCell is used correctly with proper synchronization
unsafe impl<T: Send> Send for TaskResultState<T> {}
unsafe impl<T: Send> Sync for TaskResultState<T> {}

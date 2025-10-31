use std::{cell::Cell, task::Waker};

#[cfg(not(linux))]
use std::os::fd::RawFd;

#[cfg(linux)]
pub struct OpRegistration {
  pub op: *const (),
  pub status: OpRegistrationStatus,
  pub drop_fn: fn(*const ()), // Function to properly drop the operation
}

unsafe impl Send for OpRegistration {}

#[cfg(not(linux))]
pub struct OpRegistration {
  registered_waker: Cell<Option<Waker>>,
  pub(crate) registered_listener: bool,
  pub(crate) fd: RawFd,
}

#[cfg(not(linux))]
impl OpRegistration {
  pub fn new(fd: RawFd) -> Self {
    OpRegistration {
      registered_waker: Cell::new(None),
      registered_listener: false,
      fd,
    }
  }

  pub fn wake(&mut self) {
    if let Some(wake) = self.registered_waker.take() {
      wake.wake();
    }
    self.registered_listener = false;
  }

  /// Returns "if event has been registered"
  pub fn on_event_register(&mut self, waker: Waker) -> bool {
    let old = self.registered_listener;
    self.registered_listener = true;
    self.registered_waker.set(Some(waker));

    old
  }

  pub fn has_waker(&self) -> bool {
    self.registered_listener
  }
}

#[cfg(linux)]
impl OpRegistration {
  pub fn new<T>(op: T) -> Self {
    fn drop_op<T>(ptr: *const ()) {
      drop(unsafe { Box::from_raw(ptr as *mut T) })
    }

    OpRegistration {
      op: Box::into_raw(Box::new(op)) as *const (),
      status: OpRegistrationStatus::Waiting {
        registered_waker: Cell::new(None),
      },
      drop_fn: drop_op::<T>,
    }
  }
}

#[cfg(linux)]
pub enum OpRegistrationStatus {
  Waiting {
    registered_waker: Cell<Option<Waker>>,
  },
  /// This operation is not tied to any entity waiting for it, either because they got dropped or
  /// because they weren't interested in the result.
  Cancelling,
  Done {
    ret: i32,
  },
}

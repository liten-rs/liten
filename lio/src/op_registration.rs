use std::{cell::Cell, task::Waker};

// TODO: make crossplatform with polling crate.
#[cfg(linux)]
pub struct OpRegistration {
  pub op: *const (),
  pub status: OpRegistrationStatus,
  pub drop_fn: fn(*const ()), // Function to properly drop the operation
}

unsafe impl Send for OpRegistration {}

#[cfg(not(linux))]
pub struct OpRegistration {
  pub(crate) status: OpRegistrationStatus,
  // fd: RawFd,
  // interest: EventType,
}

#[cfg(not(linux))]
impl OpRegistration {
  pub fn new_without_waker() -> Self {
    OpRegistration {
      status: OpRegistrationStatus { registered_waker: Cell::new(None) },
    }
  }

  pub fn new_with_waker(waker: Waker) -> Self {
    OpRegistration {
      status: OpRegistrationStatus { registered_waker: Cell::new(Some(waker)) },
    }
  }

  pub fn wake(&mut self) {
    if let Some(wake) = self.status.registered_waker.take() {
      wake.wake();
    }
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

#[cfg(not_linux)]
pub struct OpRegistrationStatus {
  registered_waker: Cell<Option<Waker>>,
}

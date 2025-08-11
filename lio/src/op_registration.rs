#[cfg(not(target_os = "linux"))]
use std::os::fd::RawFd;
use std::{cell::Cell, task::Waker};

#[cfg(not(target_os = "linux"))]
use crate::PollInterest;

// TODO: make crossplatform with polling crate.
#[cfg(target_os = "linux")]
pub struct OpRegistration {
  pub op: *const (),
  pub status: OpRegistrationStatus,
  pub drop_fn: fn(*const ()), // Function to properly drop the operation
}

#[cfg(not(target_os = "linux"))]
pub struct OpRegistration {
  status: OpRegistrationStatus,
  interest: PollInterest,
  fd: RawFd,
}

#[cfg(not(target_os = "linux"))]
impl OpRegistration {
  #[cfg(not(target_os = "linux"))]
  pub fn new(fd: RawFd, interest: PollInterest) -> Self {
    OpRegistration {
      status: OpRegistrationStatus { registered_waker: Cell::new(None) },
      fd,
      interest,
    }
  }

  pub fn fd(&self) -> RawFd {
    self.fd
  }

  pub fn interest(&self) -> PollInterest {
    self.interest
  }

  pub fn wake(&mut self) {
    if let Some(wake) = self.status.registered_waker.take() {
      wake.wake();
    }
  }

  pub fn set_waker(&mut self, waker: Waker) {
    self.status.registered_waker.set(Some(waker));
  }
}

#[cfg(target_os = "linux")]
impl OpRegistration {
  pub fn new<T>(op: T) -> Self {
    fn drop_op<T>(ptr: *const ()) {
      unsafe {
        let _ = Box::from_raw(ptr as *mut T);
      }
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

impl std::fmt::Debug for OpRegistration {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OpRegistration")
      .field("op", &"*const ()")
      .field("status", &self.status)
      .field("drop_fn", &"fn(*const())")
      .finish()
  }
}

#[cfg(target_os = "linux")]
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

#[cfg(not(target_os = "linux"))]
pub struct OpRegistrationStatus {
  registered_waker: Cell<Option<Waker>>,
}

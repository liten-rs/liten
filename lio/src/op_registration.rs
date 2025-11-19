use std::task::Waker;

#[cfg(not(linux))]
use std::os::fd::RawFd;

#[cfg(linux)]
use std::cell::Cell;

#[cfg(linux)]
pub struct OpRegistration {
  pub op: *const (),
  pub status: OpRegistrationStatus,
  pub drop_fn: fn(*const ()), // Function to properly drop the operation
}

unsafe impl Send for OpRegistration {}

#[cfg(not(linux))]
pub struct OpRegistration {
  registered_waker: Option<Waker>,
  pub(crate) fd: RawFd,
}

#[cfg(not(linux))]
impl OpRegistration {
  pub fn new(fd: RawFd) -> Self {
    assert!(fd != 0);
    OpRegistration { registered_waker: None, fd }
  }

  pub fn waker(&mut self) -> Option<Waker> {
    self.registered_waker.take()
  }

  /// Sets the waker, replacing any existing waker
  pub fn set_waker(&mut self, waker: Waker) {
    let _had_previous_waker = self.registered_waker.replace(waker).is_some();
    #[cfg(feature = "tracing")]
    if _had_previous_waker {
      tracing::debug!(
        fd = self.fd,
        "waker replaced (spurious poll or context change)"
      );
    }
  }

  pub fn has_waker(&self) -> bool {
    self.registered_waker.is_some()
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

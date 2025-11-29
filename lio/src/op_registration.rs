// NOTE: OpRegistration should **NEVER** impl Sync.gg
use std::task::Waker;

#[cfg(not(linux))]
use std::os::fd::RawFd;

pub struct OpCallback {
  pub callback: *const (),
  pub call_callback_fn: fn(*const (), *const (), i32),
}

unsafe impl Send for OpCallback {}

pub struct OpRegistration {
  #[cfg(linux)]
  pub status: OpRegistrationStatus,

  // Fields common to both platforms
  pub op: *const (),
  pub drop_fn: fn(*const ()), // Function to properly drop the operation
  pub callback: Option<OpCallback>,

  #[cfg(not(linux))]
  registered_waker: Option<Waker>,
  #[cfg(not(linux))]
  pub(crate) fd: RawFd,
}

unsafe impl Send for OpRegistration {}

#[cfg(linux)]
pub enum OpRegistrationStatus {
  Waiting {
    registered_waker: Option<Waker>,
  },
  /// This operation is not tied to any entity waiting for it, either because they got dropped or
  /// because they weren't interested in the result.
  Cancelling,
  Done {
    ret: i32,
  },
}

#[cfg(linux)]
impl OpRegistration {
  pub fn new<T>(op: Box<T>) -> Self {
    fn drop_op<T>(ptr: *const ()) {
      drop(unsafe { Box::from_raw(ptr as *mut T) })
    }

    OpRegistration {
      op: Box::into_raw(op) as *const (),
      status: OpRegistrationStatus::Waiting { registered_waker: None },
      drop_fn: drop_op::<T>,
      callback: None,
    }
  }
}

#[cfg(not(linux))]
impl OpRegistration {
  pub fn new<T>(op: Box<T>, fd: RawFd) -> Self {
    assert!(fd != 0);

    fn drop_op<T>(ptr: *const ()) {
      drop(unsafe { Box::from_raw(ptr as *mut T) })
    }

    OpRegistration {
      op: Box::into_raw(op) as *const (),
      drop_fn: drop_op::<T>,
      callback: None,
      registered_waker: None,
      fd,
    }
  }

  pub fn waker(&mut self) -> Option<Waker> {
    self.registered_waker.take()
  }

  /// Sets the waker, replacing any existing waker
  pub fn set_waker(&mut self, waker: Waker) {
    // Assert mutual exclusion: can't have both waker and callback
    assert!(
      self.callback.is_none(),
      "Cannot set waker when callback is already set (operation_id has callback)"
    );

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

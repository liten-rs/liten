use std::{cell::Cell, task::Waker};

pub struct OpRegistration {
  pub op: *const (),
  pub status: OpRegistrationStatus,
  pub drop_fn: fn(*const ()), // Function to properly drop the operation
}

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
impl std::fmt::Debug for OpRegistrationStatus {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Waiting { registered_waker } => f
        .debug_struct("OpRegistrationStatus::Waiting")
        .field(
          "registered_waker (is some)",
          &unsafe { &*registered_waker.as_ptr() }.is_some(),
        )
        .finish(),
      Self::Cancelling => {
        f.debug_struct("OpRegistrationStatus::Cancelling").finish()
      }
      Self::Done { ret } => {
        f.debug_struct("OpRegistrationStatus::Done").field("ret", &ret).finish()
      }
    }
  }
}

unsafe impl Send for OpRegistration {}
unsafe impl Sync for OpRegistration {}

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

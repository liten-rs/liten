// NOTE: OpRegistration should **NEVER** impl Sync.gg
use std::io;

#[cfg(not(linux))]
use std::os::fd::RawFd;
#[cfg(feature = "high")]
use std::task::Waker;

use crate::op::Operation;

pub struct OpCallback {
  callback: *const (),
  call_callback_fn: fn(*const (), &mut OpRegistration),
}
//
impl OpCallback {
  pub fn new<T, F>(callback: F) -> Self
  where
    T: Operation,
    F: FnOnce(T::Result) + Send,
  {
    OpCallback {
      callback: Box::into_raw(Box::new(callback)) as *const (),
      call_callback_fn: Self::call_callback::<T, F>,
    }
  }

  pub fn call(self, reg: &mut OpRegistration) {
    (self.call_callback_fn)(self.callback, reg);
  }

  fn call_callback<T, F>(callback_ptr: *const (), reg: &mut OpRegistration)
  where
    T: Operation,
    F: FnOnce(T::Result),
  {
    // SAFETY: We created this pointer with Box::into_raw from a Box<dyn FnOnce(T::Result) + Send>
    let callback = unsafe { Box::from_raw(callback_ptr as *mut F) };

    match reg.try_extract::<T>() {
      TryExtractOutcome::StillWaiting => panic!("wtf??"),
      TryExtractOutcome::Done(res) => callback(res),
    };
  }
}

unsafe impl Send for OpCallback {}

pub struct OpRegistration {
  status: OpRegistrationStatus,

  // Fields common to both platforms
  op: Option<*const ()>,
  op_fn_drop: fn(*const ()), // Function to properly drop the operation
  #[cfg(not(linux))]
  op_fn_run_blocking: fn(*const ()) -> std::io::Result<i32>, // Function to properly drop the operation

  #[cfg(not(linux))]
  fd: RawFd,
}

impl Drop for OpRegistration {
  fn drop(&mut self) {
    if let Some(operation) = self.op {
      (self.op_fn_drop)(operation);
    }
  }
}

unsafe impl Send for OpRegistration {}

pub enum OpRegistrationStatus {
  Waiting {
    notifier: Option<OpNotification>,
  },
  Done {
    // TODO: wtf
    // #[cfg_attr(not(feature = "high"), allow(unused))]
    // TODO: Can be optimised.
    ret: Option<io::Result<i32>>,
    // Fixes datarace when operation is done before registering callback/waker.
    before_notifier: bool,
  },
}

// Option's is for ownership rules.
pub enum OpNotification {
  #[cfg(feature = "high")]
  Waker(Option<Waker>),
  Callback(Option<OpCallback>),
}

pub enum ExtractedOpNotification {
  #[cfg(feature = "high")]
  Waker(Waker),
  Callback(OpCallback),
}

impl OpNotification {
  fn extract_notification(&mut self) -> Option<ExtractedOpNotification> {
    match self {
      #[cfg(feature = "high")]
      OpNotification::Waker(waker) => {
        let waker = waker
          .take()
          .expect("Tried to call wake on non-registered waker registration");
        #[cfg(feature = "tracing")]
        tracing::debug!("background thread: waker woken");
        Some(ExtractedOpNotification::Waker(waker))
      }
      OpNotification::Callback(call) => {
        #[cfg(feature = "tracing")]
        tracing::debug!("background thread: callback called");
        let callback = call
          .take()
          .expect("Tried to call callback on non-registered fn registration");

        Some(ExtractedOpNotification::Callback(callback))
      }
    }
  }
}

pub enum TryExtractOutcome<T = io::Result<i32>> {
  Done(T),
  StillWaiting,
}

impl OpRegistration {
  fn op_ptr(&self) -> *const () {
    self.op.expect("trying to run run_blocking after result")
  }
  #[cfg(not(linux))]
  pub(crate) fn fd(&self) -> RawFd {
    self.fd
  }

  pub fn new<T>(op: Box<T>, #[cfg(not(linux))] fd: RawFd) -> Self
  where
    T: Operation,
  {
    fn drop_op<T>(ptr: *const ()) {
      drop(unsafe { Box::from_raw(ptr as *mut T) })
    }

    #[cfg(not(linux))]
    fn op_fn_run_blocking<T>(ptr: *const ()) -> std::io::Result<i32>
    where
      T: Operation,
    {
      let op: &T = unsafe { &*(ptr as *const T) };
      op.run_blocking()
    }

    OpRegistration {
      op: Some(Box::into_raw(op) as *const ()),
      op_fn_drop: drop_op::<T>,
      #[cfg(not(linux))]
      op_fn_run_blocking: op_fn_run_blocking::<T>,
      status: OpRegistrationStatus::Waiting { notifier: None },
      #[cfg(not(linux))]
      fd,
    }
  }

  #[cfg(not(linux))]
  pub fn run_blocking(&mut self) -> io::Result<i32> {
    (self.op_fn_run_blocking)(self.op_ptr())
  }

  pub fn try_extract<T>(&mut self) -> TryExtractOutcome<T::Result>
  where
    T: Operation,
  {
    match self.status {
      OpRegistrationStatus::Waiting { notifier: _ } => {
        TryExtractOutcome::StillWaiting
      }
      OpRegistrationStatus::Done { ref mut ret, before_notifier: _ } => {
        let res = ret.take().expect("Already taken ret value after done");

        let ptr = self.op.take().expect("guarranteed not to panic, because we have owned and drop can't be called.");
        let mut op = unsafe { Box::from_raw(ptr as *mut T) };

        TryExtractOutcome::Done(op.result(res))
      }
    }
  }

  /// Sets the waker, replacing any existing waker
  #[cfg(feature = "high")]
  pub fn set_waker(&mut self, waker: Waker) {
    let notifier = match self.status {
      // Fixes datarace when operation is done before registering callback/waker.
      OpRegistrationStatus::Done { ref before_notifier, .. } => {
        if *before_notifier {
          waker.wake();
          return;
        } else {
          unreachable!("internal lio: not allowed.");
        };
      }
      OpRegistrationStatus::Waiting { ref mut notifier } => notifier,
    };

    if let Some(noti) = notifier {
      match noti {
        OpNotification::Callback(_) => {
          unreachable!("tried to set waker on callback notified operation.")
        }
        OpNotification::Waker(waker_container) => {
          // Wakers sometimes need replacing.
          let _ = waker_container.replace(waker);
        }
      }
    } else {
      *notifier = Some(OpNotification::Waker(Some(waker)));
    };
  }
  pub fn set_callback(&mut self, callback: OpCallback) {
    let notifier = match self.status {
      OpRegistrationStatus::Done { ref before_notifier, .. } => {
        if *before_notifier {
          callback.call(self);
          return;
        } else {
          unreachable!("internal lio: not allowed.");
        };
      }
      OpRegistrationStatus::Waiting { ref mut notifier } => notifier,
    };

    if let Some(noti) = notifier {
      match noti {
        OpNotification::Callback(callback_container) => {
          match callback_container {
            Some(_) => {
              panic!(
                "Tried to replace a callback on callback notified operation."
              )
            }
            None => {
              *callback_container = Some(callback);
            }
          }
        }
        #[cfg(feature = "high")]
        OpNotification::Waker(_) => {
          unreachable!("tried to set callback on waker notified operation.")
        }
      }
    } else {
      *notifier = Some(OpNotification::Callback(Some(callback)));
    };
  }

  pub fn set_done(
    &mut self,
    res: io::Result<i32>,
  ) -> Option<ExtractedOpNotification> {
    use std::mem;

    let before_notifier = match &self.status {
      OpRegistrationStatus::Waiting { notifier } => notifier.is_none(),
      OpRegistrationStatus::Done { .. } => {
        unreachable!("set done whilst already done?")
      }
    };

    let old = mem::replace(
      &mut self.status,
      OpRegistrationStatus::Done { ret: Some(res), before_notifier },
    );

    match old {
      OpRegistrationStatus::Done { .. } => {
        unreachable!("See couple of lines up unreachable statement.")
      }
      OpRegistrationStatus::Waiting { notifier } => {
        if let Some(mut notifier) = notifier {
          notifier.extract_notification()
        } else {
          None
        }
      }
    }
  }
}

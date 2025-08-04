#![allow(dead_code)]
use io_uring::IoUring;
use io_uring::types::Fd;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::marker::PhantomData;
use std::mem;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

type OperationId = u64;

pub struct OpRegistration {
  op: *const (),
  status: OpRegistrationStatus,
  drop_fn: fn(*const ()), // Function to properly drop the operation
}

unsafe impl Send for OpRegistration {}
unsafe impl Sync for OpRegistration {}

impl Debug for OpRegistration {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OpRegistration")
      .field("op", &"*const ()")
      .field("status", &self.status)
      .field("drop_fn", &"fn(*const())")
      .finish()
  }
}

pub enum OpRegistrationStatus {
  Waiting { registered_waker: Cell<Option<Waker>> },
  Cancelling,
  Done { ret: i32 },
}

unsafe impl Sync for OpRegistrationStatus {}

impl Debug for OpRegistrationStatus {
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

impl OpRegistration {
  pub fn wake_registered(&self) {
    if let OpRegistrationStatus::Waiting { ref registered_waker } = self.status
    {
      if let Some(waker) = registered_waker.take() {
        waker.wake_by_ref();
      }
    }
  }
}

fn main() {}

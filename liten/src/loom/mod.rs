#![allow(unused_imports)]

mod unsafe_cell;

pub(crate) mod cell {

  #[cfg(not(loom))]
  pub(crate) use super::unsafe_cell::UnsafeCell;

  #[cfg(loom)]
  pub(crate) use loom::cell::UnsafeCell;

  #[cfg(not(loom))]
  pub(crate) use std::cell::Cell;

  #[cfg(loom)]
  pub(crate) use loom::cell::Cell;
}

pub(crate) mod sync {
  #[cfg(loom)]
  pub use loom::sync::{Arc, Mutex, MutexGuard, RwLock};
  #[cfg(not(loom))]
  pub use std::sync::{Arc, Mutex, MutexGuard, RwLock};

  #[cfg(loom)]
  pub use loom::sync::mpsc;
  #[cfg(not(loom))]
  pub use std::sync::mpsc;

  pub mod atomic {
    #[cfg(loom)]
    pub use loom::sync::atomic::{
      AtomicBool, AtomicPtr, AtomicU16, AtomicU8, AtomicUsize, Ordering,
    };
    #[cfg(not(loom))]
    pub use std::sync::atomic::{
      AtomicBool, AtomicPtr, AtomicU16, AtomicU8, AtomicUsize, Ordering,
    };
  }
}

#[cfg(loom)]
pub use loom::thread;

#[cfg(not(loom))]
pub use std::thread;

#[cfg(loom)]
pub use loom::thread_local;

#[cfg(not(loom))]
pub use std::thread_local;

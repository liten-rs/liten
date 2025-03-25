#![allow(unused_imports)]

mod unsafe_cell;

pub(crate) mod cell {

  #[cfg(not(loom))]
  pub(crate) use super::unsafe_cell::UnsafeCell;

  #[cfg(loom)]
  pub(crate) use loom::cell::UnsafeCell;
}

pub(crate) mod sync {
  #[cfg(loom)]
  pub use loom::sync::{Arc, Mutex, MutexGuard, RwLock};
  #[cfg(not(loom))]
  pub use std::sync::{Arc, Mutex, MutexGuard, RwLock};

  pub mod atomic {
    #[cfg(loom)]
    pub use loom::sync::atomic::{
      AtomicBool, AtomicU16, AtomicU8, AtomicUsize, Ordering,
    };
    #[cfg(not(loom))]
    pub use std::sync::atomic::{
      AtomicBool, AtomicU16, AtomicU8, AtomicUsize, Ordering,
    };
  }
}

pub(crate) mod thread {
  #[cfg(loom)]
  pub use loom::thread::{current, park, spawn, Builder, JoinHandle, Thread};

  #[cfg(not(loom))]
  pub use std::thread::{current, park, spawn, Builder, JoinHandle, Thread};
}

#[cfg(loom)]
pub use loom::thread_local;

#[cfg(not(loom))]
pub use std::thread_local;

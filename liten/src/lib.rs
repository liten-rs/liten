#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../../book/src/SUMMARY.md")]

#[macro_use]
mod macros;
mod loom;

pub mod future;

pub(crate) mod data;
#[doc(hidden)]
pub mod testing_util;

#[doc(hidden)]
pub use liten_macros::{internal_test, runtime_test};

cfg_blocking! {
  pub mod blocking;
}

cfg_fs! {
  pub mod fs;
}

pub mod io;

cfg_rt! {
  pub mod runtime;
  pub use liten_macros::{main, test};
}

cfg_sync! {
  pub mod sync;
}
pub mod task;

cfg_time! {
  pub mod time;
}

cfg_actor! {
  pub mod actor;
}

#[doc(hidden)]
pub mod utils;

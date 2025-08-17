#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../../book/src/introduction.md")]

#[macro_use]
mod macros;
mod loom;

cfg_coro! {
  pub mod coro;
}

pub mod future;

pub(crate) mod data;
#[doc(hidden)]
pub mod testing_util;

#[doc(hidden)]
pub use liten_macros::internal_test;

cfg_blocking! {
  pub mod blocking;
}

cfg_fs! {
  pub mod fs;
}

#[cfg(unix)]
cfg_io! {
  pub mod io;
}

cfg_rt! {
  pub mod runtime;
  pub use liten_macros::{main, test};
}

cfg_sync! {
  pub mod sync;
}

pub mod task;

#[cfg(not(loom))]
cfg_time! {
  pub mod time;
}

mod parking;

cfg_compat! {
  pub mod compat;
}

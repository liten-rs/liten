#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../../book/src/introduction.md")]

#[macro_use]
pub mod macros;
mod loom;

pub mod future;

// pub(crate) mod data;
#[doc(hidden)]
pub mod testing_util;

#[doc(hidden)]
pub use liten_macros::internal_test;

cfg_rt! {
  pub mod runtime;
  pub use liten_macros::{main, test};

  pub fn block_on<F>(f: F) -> F::Output
  where
    F: std::future::Future,
  {
    runtime::Runtime::single_threaded().block_on(f)
  }
}

cfg_sync! {
  pub mod sync;
}

pub mod task;

// #[cfg(not(loom))]
// cfg_time! {
//   pub mod time;
// }

// mod parking;

// cfg_compat! {
//   pub mod compat;
// }

#![doc = include_str!("../../README.md")]

pub use liten_macros::internal_test;
pub use liten_macros::{main, test};

#[cfg(feature = "blocking")]
pub mod blocking;

mod context;
mod events;
#[cfg(feature = "fs")]

pub mod fs;
pub mod io;
mod loom;
pub mod runtime;

#[cfg(feature = "sync")]
pub mod sync;
#[cfg(not(feature = "sync"))]
mod sync;

pub mod task;

#[cfg(feature = "time")]
pub mod time;

#[cfg(feature = "actor")]
pub mod actor;

pub mod macros;
pub mod utils;


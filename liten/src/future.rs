pub(crate) mod block_on;
pub mod map;
pub mod or;
mod util;

#[cfg(feature = "time")]
pub mod timeout;

pub use block_on::block_on;

#[cfg(feature = "runtime")]
use crate::task::{self, TaskHandle};

use std::future::Future;

use crate::future::{map::Map, or::Or};

/// Extension traits and utilities for working with [`Future`]s.
///
/// This module provides additional combinators and helpers for futures,
/// such as the ability to add timeouts to futures via the [`FutureExt`] trait.
///
/// # Features
///
/// - The `timeout` method is available when the `time` feature is enabled.
///
/// # Examples
///
/// ```ignore
/// use liten::future::FutureExt;
/// use std::time::Duration;
///
/// async fn my_async_fn() {
///     // some async work
/// }
///
/// # async fn example() {
/// let result = my_async_fn().timeout(Duration::from_secs(5)).await;
/// match result {
///     Ok(val) => println!("Completed: {:?}", val),
///     Err(e) => println!("Timed out: {}", e),
/// }
/// # }
/// ```
pub trait FutureExt: Future {
  cfg_rt! {
    /// Spawns the future on the runtime. See [crate::task]
    fn spawn(self) -> TaskHandle<Self::Output>
    where
      Self: Sized + Send + 'static,
      Self::Output: Send,
    {
      task::spawn(self)
    }
  }

  /// Modify the final output of the future.
  fn map<Fun: FnOnce(Self::Output) -> R, R>(self, fun: Fun) -> Map<Self, Fun>
  where
    Self: Sized,
  {
    Map::new(self, fun)
  }

  /// Returns the future which is done first.
  fn or<F2: Future, Out>(self, fut: F2) -> Or<Self, F2, Out>
  where
    Self: Sized,
  {
    Or::new(self, fut)
  }

  #[cfg(not(loom))]
  cfg_time! {
    /// Start awaiting the future but only before the timeout, after it cancels.
    fn timeout(
      self,
      duration: std::time::Duration,
    ) -> impl Future<Output = Result<Self::Output, timeout::Timeout>>
    where
      Self: Sized,
    {
      Or::new(
       self.map(Ok),
       Map::new(crate::time::sleep(duration), |_| Err(timeout::Timeout))
      )
    }
  }
}

impl<F: Future> FutureExt for F {}

pub trait Stream {
  type Item: Sized;
  fn next(&self) -> impl Future<Output = Option<Self::Item>>;
}

pub(crate) mod block_on;
pub mod map;
pub mod or;
mod util;
pub use block_on::block_on;

cfg_time! {
  use std::time::Duration;
  pub mod timeout;
}

use std::future::Future;

use crate::{
  future::{map::Map, or::Or},
  task::{self, TaskHandle},
};

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
  /// Spawns the future on the runtime. See [crate::task]
  fn spawn(self) -> TaskHandle<Self::Output>
  where
    Self: Sized + Send + 'static,
    Self::Output: Send,
  {
    task::spawn(self)
  }

  /// Start awaiting the future but only before the timeout, after it cancels.
  fn map<Fun: FnOnce(Self::Output) -> R, R>(self, fun: Fun) -> Map<Self, Fun>
  where
    Self: Sized,
  {
    Map::new(self, fun)
  }

  /// Start awaiting the future but only before the timeout, after it cancels.
  fn or<F2: Future, Out>(self, fut: F2) -> Or<Self, F2, Out>
  where
    Self: Sized,
  {
    Or::new(self, fut)
  }

  cfg_time! {
    /// Start awaiting the future but only before the timeout, after it cancels.
    fn timeout(
      self,
      duration: Duration,
    ) -> impl Future<Output = Result<Self::Output, timeout::Timeout>>
    where
      Self: Sized + Send,
    {
      Or::new(
       self.map(Ok),
       Map::new(crate::time::sleep(duration), |_| Err(timeout::Timeout))
      )
    }
  }
}

impl<F: Future> FutureExt for F {}

pub(crate) mod block_on;
pub use block_on::block_on;

cfg_time! {
  use std::time::Duration;
  mod timeout;
}

use std::future::Future;

use crate::task::{self, TaskHandle};

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
  cfg_time! {
    /// Start awaiting the future but only before the timeout, after it cancels.
    fn timeout(
      self,
      duration: Duration,
    ) -> impl Future<Output = Result<Self::Output, timeout::Timeout>> + Send
    where
      Self: Sized + Send,
    {
      use crate::future::timeout::FutureTimeout;
      FutureTimeout { future: self, sleep_future: crate::time::sleep(duration) }
    }
  }
}

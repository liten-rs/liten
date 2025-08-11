/// Error returned when a future times out.
///
/// This error is produced by the [`FutureExt::timeout`](super::FutureExt::timeout) method when the inner future does not complete within the specified duration.
#[cfg(feature = "time")]
#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Timeout reached")]
pub struct Timeout;

#[cfg(test)]
mod tests {
  use super::Timeout;
  use std::future::ready;
  use std::time::Duration;

  #[crate::internal_test]
  #[cfg(feature = "time")]
  fn future_timeout_completes_before_timeout() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      use crate::future::FutureExt;

      assert!(FutureExt::timeout(ready(0), Duration::from_millis(100))
        .await
        .is_ok());
    })
  }

  cfg_time! {
    #[crate::internal_test]
      #[cfg(not(loom))] // runs so many times so 100ms * many runs never completes
    fn future_timeout_fires_on_sleep() {
        crate::runtime::Runtime::single_threaded().block_on(async {
            use crate::future::FutureExt;
            use std::time::Duration;

            // This future never completes
            let result = std::future::pending::<Result<(), Timeout>>().timeout(Duration::from_millis(100)).await;
            assert_eq!(result, Err(Timeout));
        })
    }
  }
}

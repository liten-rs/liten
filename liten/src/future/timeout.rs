/// Error returned when a future times out.
///
/// This error is produced by the [`FutureExt::timeout`] method when the inner future does not complete within the specified duration.
#[cfg(feature = "time")]
#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Timeout reached")]
pub struct Timeout;

#[cfg(test)]
mod tests {
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

  // #[crate::internal_test]
  // #[cfg(feature = "time")]
  // fn future_timeout_completes_after_timeout() {
  //   crate::runtime::Runtime::single_threaded().block_on(async {
  //     use crate::{future::FutureExt, time::sleep};
  //
  //     assert!(FutureExt::timeout(
  //       sleep(Duration::from_millis(1000)),
  //       Duration::from_millis(10),
  //     )
  //     .await
  //     .is_err());
  //   })
  // }
}

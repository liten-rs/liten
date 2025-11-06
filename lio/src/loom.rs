pub mod sync {
  // When using loom for concurrency testing
  #[cfg(loom)]
  pub use loom::sync::{Arc, Mutex};

  #[cfg(loom)]
  pub use std::sync::OnceLock;

  // Normal runtime (not loom)
  #[cfg(not(loom))]
  pub use std::sync::{Arc, OnceLock};

  #[cfg(not(loom))]
  pub use parking_lot::Mutex;

  pub mod atomic {
    #[cfg(loom)]
    pub use loom::sync::atomic::*;

    #[cfg(not(loom))]
    pub use std::sync::atomic::*;
  }
}

// Thread module
#[cfg(loom)]
pub use loom::thread;

#[cfg(not(loom))]
pub use std::thread;

// Test utilities that abstract over different test modes
pub mod test_utils {
  use std::future::Future;

  // ============================================================================
  // Async runtime abstraction
  // ============================================================================

  /// Run an async function to completion
  ///
  /// This abstracts over different test modes:
  /// - Normal mode: Uses tokio
  /// - Loom mode: Uses loom's futures executor
  pub fn block_on<F, O>(fut: F) -> O
  where
    F: Future<Output = O>,
  {
    #[cfg(not(loom))]
    {
      tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
    }

    #[cfg(loom)]
    {
      loom::future::block_on(fut)
    }
  }

  // ============================================================================
  // Task spawning abstraction
  // ============================================================================

  #[cfg(test)]
  pub use tokio::task::spawn;

  // ============================================================================
  // LocalPool abstraction for single-threaded executor
  // ============================================================================

  // ============================================================================
  // Sleep abstraction
  // ============================================================================

  /// Sleep for a duration - useful for tests
  pub fn sleep(duration: std::time::Duration) {
    #[cfg(not(loom))]
    {
      std::thread::sleep(duration);
    }

    #[cfg(loom)]
    {
      // Loom doesn't support sleep, yield instead
      loom::thread::yield_now();
    }
  }

  // ============================================================================
  // Model checking wrapper for loom
  // ============================================================================

  /// Run a test under loom model checking or normally
  ///
  /// Usage:
  /// ```
  /// model(|| {
  ///     // test code here
  /// });
  /// ```
  pub fn model<F>(f: F)
  where
    F: Fn() + Sync + Send + 'static,
  {
    #[cfg(loom)]
    {
      loom::model(f);
    }

    #[cfg(not(loom))]
    {
      // Just run once in non-loom mode
      f();
    }
  }
}

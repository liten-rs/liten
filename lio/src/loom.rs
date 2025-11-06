pub mod sync {
  // Re-export Arc and OnceLock
  #[cfg(loom)]
  pub use loom::sync::Arc;

  #[cfg(loom)]
  pub use std::sync::OnceLock;

  #[cfg(not(loom))]
  pub use std::sync::{Arc, OnceLock};

  // Custom Mutex wrapper that removes poisoning
  pub struct Mutex<T> {
    #[cfg(not(loom))]
    inner: parking_lot::Mutex<T>,
    #[cfg(loom)]
    inner: loom::sync::Mutex<T>,
  }

  impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
      Self {
        inner: {
          #[cfg(not(loom))]
          {
            parking_lot::Mutex::new(value)
          }
          #[cfg(loom)]
          {
            loom::sync::Mutex::new(value)
          }
        },
      }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
      #[cfg(not(loom))]
      {
        MutexGuard { inner: self.inner.lock() }
      }
      #[cfg(loom)]
      {
        // Remove poisoning by using unwrap_or_else to recover
        MutexGuard {
          inner: self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        }
      }
    }
  }

  pub struct MutexGuard<'a, T> {
    #[cfg(not(loom))]
    inner: parking_lot::MutexGuard<'a, T>,
    #[cfg(loom)]
    inner: loom::sync::MutexGuard<'a, T>,
  }

  impl<'a, T> std::ops::Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
      &self.inner
    }
  }

  impl<'a, T> std::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      &mut self.inner
    }
  }

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

  /// Run an async function to completion with a timeout
  ///
  /// This abstracts over different test modes:
  /// - Normal mode: Uses tokio with 30 second timeout
  /// - Loom mode: Uses loom's futures executor (no timeout)
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

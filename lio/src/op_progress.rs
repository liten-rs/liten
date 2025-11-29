#[cfg(all(linux, feature = "high"))]
use crate::driver::CheckRegistrationResult;
#[cfg(not(linux))]
use crate::op::Operation;

#[cfg(feature = "high")]
use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use crate::{
  Driver,
  op::{self, DetachSafe},
};

/// Represents the progress of an I/O operation across different platforms.
///
/// This enum provides a unified interface for tracking I/O operations regardless
/// of the underlying platform implementation. It automatically selects the most
/// efficient execution method for each platform.
///
/// # Platform-Specific Behavior
///
/// - **Linux**: Uses io_uring for maximum performance when supported
/// - **Other platforms**: Falls back to polling-based async I/O or blocking execution
///
/// # Examples
///
/// ```rust
/// use lio::{read, OperationProgress};
/// use std::os::fd::RawFd;
///
/// async fn example() -> std::io::Result<()> {
///     let fd: RawFd = 0; // stdin
///     let buffer = vec![0u8; 1024];
///     
///     let progress: OperationProgress<lio::op::Read> = read(fd, buffer, 0);
///     let (result_bytes_read, buf) = progress.await;
///     
///     println!("Read {} bytes", result_bytes_read?);
///     Ok(())
/// }
/// ```
pub enum OperationProgress<T>
where
  T: op::Operation,
{
  #[cfg(not(linux))]
  #[cfg_attr(docsrs, doc(cfg(not(linux))))]
  Poll {
    event: polling::Event,
    id: u64,
  },

  #[cfg(linux)]
  #[cfg_attr(docsrs, doc(cfg(linux)))]
  IoUring {
    id: u64,
  },

  Blocking {
    operation: Option<T>,
  },
}

unsafe impl<T> Send for OperationProgress<T> where T: Send + op::Operation {}

impl<T: op::Operation> OperationProgress<T> {
  /// Detaches this progress tracker from the driver without binding it to any object.
  ///
  /// This function is useful when you want to clean up the operation registration
  /// without waiting for the operation to complete. It's automatically called
  /// when the `OperationProgress` is dropped.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::{read, OperationProgress};
  /// use std::os::fd::RawFd;
  ///
  /// async fn detach_example() -> std::io::Result<()> {
  ///     let fd: RawFd = 0;
  ///     let buffer = vec![0u8; 1024];
  ///     let progress: OperationProgress<lio::op::Read> = read(fd, buffer, 0);
  ///     
  ///     // Detach without waiting for completion
  ///     progress.detach();
  ///     
  ///     Ok(())
  /// }
  /// ```
  pub fn detach(self)
  where
    T: DetachSafe + 'static,
  {
    self.when_done(drop);
  }

  /// Registers a callback to be invoked when the operation completes.
  ///
  /// This method takes ownership of the `OperationProgress`, preventing it from being
  /// polled as a Future. The callback will receive the operation result when the I/O
  /// operation completes.
  ///
  /// # Mutual Exclusion with Future Polling
  ///
  /// Once `when_done` is called, the operation cannot be polled as a Future. This is
  /// enforced by taking ownership of `self`. You must choose one execution model:
  /// - **Await the Future**: Use `.await` to get the result synchronously in your async code
  /// - **Use a callback**: Use `.when_done()` for fire-and-forget operations or when you
  ///   need the result in a different context
  ///
  /// # Callback Requirements
  ///
  /// The callback must be `FnOnce(T::Result) + Send + 'static`:
  /// - `FnOnce`: The callback is invoked exactly once when the operation completes
  /// - `Send`: The callback may be executed on a background I/O thread
  /// - `'static`: The callback must not borrow data with lifetimes (use `move` closures
  ///   with owned data or `Arc`/`Arc<Mutex<T>>` for shared state)
  ///
  /// # Platform-Specific Behavior
  ///
  /// - **Blocking operations**: The callback is invoked immediately (synchronously)
  /// - **Async operations** (io_uring/polling): The callback is invoked asynchronously
  ///   on the background I/O thread when the operation completes
  ///
  /// # Examples
  ///
  /// ## Basic callback usage
  ///
  /// ```rust
  /// use lio::read;
  /// use std::sync::mpsc::channel;
  ///
  /// async fn example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let buffer = vec![0u8; 1024];
  ///     let (tx, rx) = channel();
  ///
  ///     // Use callback instead of awaiting
  ///     read(fd, buffer, 0).when_done(move |(result, buf)| {
  ///         match result {
  ///             Ok(bytes_read) => {
  ///                 println!("Read {} bytes", bytes_read);
  ///                 tx.send(buf).unwrap();
  ///             }
  ///             Err(e) => eprintln!("Error: {}", e),
  ///         }
  ///     });
  ///
  ///     // Continue with other work while I/O happens in background
  ///     // ...
  ///
  ///     // Later, wait for the result
  ///     let buffer = rx.recv().unwrap();
  ///     Ok(())
  /// }
  /// ```
  ///
  /// ## Shared state with Arc
  ///
  /// ```rust
  /// use lio::write;
  /// use std::sync::{Arc, Mutex};
  ///
  /// async fn example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let data = b"Hello, callbacks!".to_vec();
  ///     let result = Arc::new(Mutex::new(None));
  ///     let result_clone = result.clone();
  ///
  ///     write(fd, data, 0).when_done(move |(bytes_written, _buf)| {
  ///         *result_clone.lock().unwrap() = Some(bytes_written);
  ///     });
  ///
  ///     // Continue with other work...
  ///     Ok(())
  /// }
  /// ```
  pub fn when_done<F>(mut self, callback: F)
  where
    F: FnOnce(T::Result) + Send + 'static,
  {
    match self {
      #[cfg(linux)]
      OperationProgress::IoUring { id, .. } => {
        Driver::get().set_callback::<T>(id, Box::new(callback));
        std::mem::forget(self); // Prevent Drop from cancelling the operation
      }
      #[cfg(not(linux))]
      OperationProgress::Poll { id, .. } => {
        Driver::get().set_callback::<T>(id, Box::new(callback));
        std::mem::forget(self); // Prevent Drop from cancelling the operation
      }
      OperationProgress::Blocking { ref mut operation } => {
        // For blocking operations, run immediately and invoke callback
        let mut op =
          operation.take().expect("Blocking operation already consumed");
        let result = op.run_blocking();
        let output = op.result(result);
        callback(output);
      }
    }
  }

  #[cfg(feature = "high")]
  pub fn get_receiver(self) -> oneshot::Receiver<T::Result>
  where
    T::Result: Send + 'static,
  {
    let (sender, receiver) = oneshot::channel();

    self.when_done(move |res| {
      let _ = sender.send(res);
    });

    receiver
  }
}

#[cfg(linux)]
impl<T> OperationProgress<T>
where
  T: op::Operation,
{
  pub(crate) fn new_uring(id: u64) -> Self {
    Self::IoUring { id }
  }

  pub(crate) fn new_blocking(op: T) -> Self {
    Self::Blocking { operation: Some(op) }
  }
}

#[cfg(not(linux))]
impl<T> OperationProgress<T>
where
  T: op::Operation,
{
  pub(crate) fn new(id: u64) -> Self
  where
    T: Operation,
  {
    let test = match T::EVENT_TYPE {
      None => panic!(
        "tried running OperationProgress::new without associated op event type"
      ),
      Some(test) => test,
    };
    use crate::op::EventType;

    let event = match test {
      EventType::Read => polling::Event::readable(id as usize),
      EventType::Write => polling::Event::writable(id as usize),
    };

    Self::Poll { id, event }
  }

  pub(crate) fn new_blocking(operation: T) -> Self {
    Self::Blocking { operation: Some(operation) }
  }
}

/// Implements `Future` for io_uring-based operations on Linux.
///
/// This implementation handles the completion of operations submitted to the
/// io_uring subsystem, automatically waking the future when the operation
/// completes.
#[cfg(all(feature = "high", linux))]
#[cfg_attr(docsrs, doc(cfg(feature = "high")))]
impl<T> Future for OperationProgress<T>
where
  T: op::Operation + Unpin,
{
  type Output = T::Result;

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let result = match *self {
      OperationProgress::IoUring { ref id } => {
        let is_done = Driver::get()
          .check_registration::<T>(*id, cx.waker().clone())
          .expect("Polled OperationProgress when not even registered");

        match is_done {
          CheckRegistrationResult::WakerSet => Poll::Pending,
          CheckRegistrationResult::Value(result) => Poll::Ready(result),
        }
      }
      OperationProgress::Blocking { ref mut operation } => {
        let mut op =
          operation.take().expect("Blocking operation polled after completion");
        let result = op.run_blocking();
        Poll::Ready(op.result(result))
      }
    };

    result
  }
}

/// Implements `Future` for polling-based operations on non-Linux platforms.
///
/// This implementation handles operations that use polling-based async I/O,
/// automatically re-registering for events when operations would block.
#[cfg(all(feature = "high", not(linux)))]
#[cfg_attr(docsrs, doc(cfg(feature = "high")))]
impl<T> Future for OperationProgress<T>
where
  T: op::Operation + Unpin,
{
  type Output = T::Result;

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let result = match *self {
      OperationProgress::Blocking { ref mut operation } => {
        let mut op =
          operation.take().expect("Blocking operation polled after completion");
        let result = op.run_blocking();
        Poll::Ready(op.result(result))
      }
      OperationProgress::Poll { id, event } => {
        // Delegate to Driver to execute the operation
        Driver::get().try_execute_operation::<T>(id, event, cx.waker().clone())
      }
    };

    result
  }
}

#[cfg(not(linux))]
#[cfg_attr(docsrs, doc(auto_cfg = false))]
impl<T> Drop for OperationProgress<T>
where
  T: op::Operation,
{
  fn drop(&mut self) {
    match self {
      OperationProgress::Poll { id, .. } => {
        Driver::get().detach(*id);
      }
      OperationProgress::Blocking { .. } => {
        // Blocking operations don't need cleanup
      }
    }
  }
}

/// Implements automatic cleanup for io_uring operations on Linux.
///
/// When an `OperationProgress` is dropped, this implementation ensures
/// that the operation is properly cancelled and cleaned up from the driver.
#[cfg(linux)]
#[cfg_attr(docsrs, doc(auto_cfg = false))]
impl<T> Drop for OperationProgress<T>
where
  T: op::Operation,
{
  fn drop(&mut self) {
    match self {
      OperationProgress::IoUring { id, .. } => {
        Driver::get().detach(*id);
      }
      OperationProgress::Blocking { .. } => {
        // Blocking operations don't need cleanup
      }
    }
  }
}

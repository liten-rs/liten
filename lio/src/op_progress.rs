#[cfg(linux)]
use crate::driver::CheckRegistrationResult;
#[cfg(not_linux)]
use crate::op::Operation;
#[cfg(linux)]
use std::marker::PhantomData;

use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use crate::{Driver, op};

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
///     let (bytes_read, buf) = progress.await?;
///     
///     println!("Read {} bytes", bytes_read);
///     Ok(())
/// }
/// ```
#[cfg(linux)]
pub enum OperationProgress<T> {
  IoUring { id: u64, _m: PhantomData<T> },
  Blocking { operation: T },
}

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
///     let (bytes_read, buf) = progress.await?;
///     
///     println!("Read {} bytes", bytes_read);
///     Ok(())
/// }
/// ```
#[cfg(not(linux))]
pub enum OperationProgress<T> {
  Poll { event: polling::Event, id: u64, operation: T },
  Blocking { operation: T },
}

impl<T> OperationProgress<T> {
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
  pub fn detach(self) {
    // Engages the Driver::detach(..)
    drop(self);
  }
}

#[cfg(linux)]
impl<T> OperationProgress<T> {
  pub fn new_uring(id: u64) -> Self {
    Self::IoUring { id, _m: PhantomData }
  }

  pub fn new_blocking(op: T) -> Self {
    Self::Blocking { operation: op }
  }
}

#[cfg(not(linux))]
impl<T> OperationProgress<T> {
  pub fn new(id: u64, operation: T) -> Self
  where
    T: Operation,
  {
    if let Some(test) = T::EVENT_TYPE {
      use crate::op::EventType;

      let event = match test {
        EventType::Read => polling::Event::readable(id as usize),
        EventType::Write => polling::Event::writable(id as usize),
      };

      Self::Poll { id, event, operation }
    } else {
      Self::Blocking { operation }
    }
  }

  pub fn new_blocking(operation: T) -> Self {
    Self::Blocking { operation }
  }
}

/// Implements `Future` for io_uring-based operations on Linux.
///
/// This implementation handles the completion of operations submitted to the
/// io_uring subsystem, automatically waking the future when the operation
/// completes.
#[cfg(linux)]
impl<T> Future for OperationProgress<T>
where
  T: op::Operation + Unpin,
{
  type Output = T::Result;

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    match *self {
      OperationProgress::IoUring { ref id, ref _m } => {
        let is_done = Driver::get()
          .check_registration::<T>(*id, cx.waker().clone())
          .expect("Polled OperationProgress when not even registered");

        match is_done {
          CheckRegistrationResult::WakerSet => Poll::Pending,
          CheckRegistrationResult::Value(result) => Poll::Ready(result),
        }
      }
      OperationProgress::Blocking { ref mut operation } => {
        let result = operation.run_blocking();
        Poll::Ready(operation.result(result))
      }
    }
  }
}

/// Implements `Future` for polling-based operations on non-Linux platforms.
///
/// This implementation handles operations that use polling-based async I/O,
/// automatically re-registering for events when operations would block.
#[cfg(not(linux))]
impl<T> Future for OperationProgress<T>
where
  T: op::Operation + Unpin,
{
  type Output = T::Result;

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    match *self {
      OperationProgress::Poll { id, ref mut operation, event } => {
        use std::io;

        match operation.run_blocking() {
          Ok(result) => Poll::Ready(operation.result(Ok(result))),
          Err(err) => {
            if err.kind() == io::ErrorKind::WouldBlock
              || err.raw_os_error() == Some(libc::EINPROGRESS)
            {
              let result = Driver::get()
                .register_repoll(
                  id,
                  event,
                  cx.waker().clone(),
                  operation.fd().expect(
                    "operation has event_type.is_some(), but not fd Some(...)",
                  ),
                )
                .expect("why didn't exist");
              if let Err(err) = result {
                Poll::Ready(operation.result(Err(err)))
              } else {
                Poll::Pending
              }
            } else {
              Poll::Ready(operation.result(Err(err)))
            }
          }
        }
      }
      OperationProgress::Blocking { ref mut operation } => {
        let result = operation.run_blocking();
        Poll::Ready(operation.result(result))
      }
    }
  }
}

/// Implements automatic cleanup for io_uring operations on Linux.
///
/// When an `OperationProgress` is dropped, this implementation ensures
/// that the operation is properly cancelled and cleaned up from the driver.
#[cfg(linux)]
impl<T> Drop for OperationProgress<T> {
  fn drop(&mut self) {
    if let OperationProgress::IoUring { id, .. } = *self {
      Driver::get().detach(id);
    }
  }
}

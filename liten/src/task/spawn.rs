use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};
use thiserror::Error;

use crate::sync::oneshot;

use super::{builder, Builder, TaskHandle};

/// Spawns a new asynchronous task.
///
/// A task is a lightweight unit of execution that runs concurrently with other tasks.
/// When you spawn a task, it begins executing immediately in the background without
/// blocking the current execution context. The point of tasks is to run when progress can be made,
/// and let other tasks run when this one cannot.
///
/// # Examples
///
/// Basic spawning:
/// ```rust
/// use liten::task;
///
/// #[liten::main]
/// async fn main() {
///     let handle = task::spawn(async {
///         // This work runs concurrently
///         std::thread::sleep(std::time::Duration::from_millis(100));
///         42
///     });
///
///     // Do other work while the task runs
///     println!("Task is running in background");
///
///     // Wait for the task to complete
///     let result = handle.await.unwrap();
///     assert_eq!(result, 42);
/// }
/// ```
///
/// Spawning multiple tasks:
/// ```rust
/// use liten::task;
///
/// #[liten::main]
/// async fn main() {
///
///     // Wait for all tasks to complete
///     let results = liten::join!(
///       task::spawn(async move { 1u8 }),
///       task::spawn(async move { 2u8 })
///     );
///
///     assert_eq!(results, (Ok(1u8), Ok(2u8)));
/// }
/// ```
///
/// # Panics
///
/// If the spawned task panics, the `TaskHandle` will return a `TaskHandleError::BodyPanicked`
/// when awaited.
///
/// # Returns
///
/// Returns a `TaskHandle` that can be awaited to get the result of the spawned task.
/// The handle implements `IntoFuture`, so you can directly await it or use it with
/// other async combinators.
pub fn spawn<F>(fut: F) -> TaskHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send,
{
  Builder::default().spawn(fut)
}

//! Task/Green-thread
//!
//! A task is the smallest unit of asynchronous computation in liten. A task runs concurrently with other
//! tasks and they only run when progress can be made. In other words, compute only gets available
//! to tasks that needs it to move forwards.
//!
//! For example, if a web server is run, and no connections is coming in, no
//! cpu will be used. And when issuing a db-call through the network, the executor knows that no
//! work is needed (since we are waiting for the network), so it runs other tasks that can utilize cpu.
//!

#![allow(clippy::module_inception)]
mod task;
pub use task::*;
mod yield_now;
use std::future::Future;

/// Spawns a new asynchronous task.
///
/// *See [task module docs](crate::task) for documentation about tasks*
///
/// When you spawn a task, it behaves a bit differently depending on which scheduler you are using.
///
/// ## Multithreaded executor
/// When a multithreaded executor is running, tasks get polled by worker threads in the background.
/// This means that even if your main thread blocks, tasks can get executed.
///
/// ## Singlethreaded executor
/// When a singlethreaded executor is running, tasks are polled when the main function
/// doesn't block, in other words, waiting for an await call to finish.
///
///
/// Tasks can return values and be retreived if needed by the caller by awaiting the
/// [`TaskHandle`](`crate::task::TaskHandle`). TaskHandle returns the value wrapped by a [`Result`]
/// because the future being able to panic.
///
/// # Examples
/// ```rust
/// use liten::task::{self, TaskHandle};
///
/// # #[liten::main]
/// # async fn main() {
///   let handle: TaskHandle<u8> = task::spawn(async {
///     // This work runs concurrently
///
///     // lots of work here.
///     42
///   });
///   
///   // Do other work while the task runs
///   println!("Task is running in background");
///   
///   // Wait for the task to complete
///   // the future can panic inside the task, hence the .unwrap()
///   let result = handle.await.unwrap();
///   assert_eq!(result, 42);
/// # }
/// ```
///
/// If multiple tasks are present and need to finish at the same time, the [`join!`](`crate::join`) macro can be
/// used
///
/// Spawning multiple tasks:
/// ```rust
/// use liten::task;
///
/// # #[liten::main]
/// # async fn main() {
///
///     // Wait for all tasks to complete
///     let results = liten::join!(
///       task::spawn(async move { 1u8 }),
///       task::spawn(async move { 2u8 })
///     );
///
///     assert_eq!(results, (Ok(1u8), Ok(2u8)));
/// # }
/// ```
///
/// In this case, the futures can have different output types.
///
/// # Panics
///
/// If the spawned task panics, the `TaskHandle` will return a [`TaskHandleError::BodyPanicked`](`crate::task::TaskHandleError::BodyPanicked`)
/// when awaited.
///
/// # Returns
///
/// Returns a `TaskHandle` that can be awaited to get the result of the spawned task.
/// The handle implements `Future`, so you can directly await it or use it with
/// other async combinators.
#[cfg(feature = "runtime")]
pub fn spawn<F>(fut: F) -> task::TaskHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send,
{
  let store = task::TaskStore::get();
  let (task, handle) = task::Task::new(fut);

  store.task_enqueue(task);

  handle
}

/// Give up some time to the task scheduler.
///
/// *See [task module docs](crate::task) for documentation about tasks*
///
/// When a task/main future currently on has little to do, or for some reason is waiting for another task
/// but doesn't have access to the handle, it can signal to the schedule that the caller is willing
/// to give up compute to prioritize other tasks.
///
/// A drawback is that if no other tasks are running, this will just busy-wait for a while, which
/// wastes CPU and energy.
///
/// Other primitives should be used instead of (checkout [`sync`](`crate::sync`) for async primitives) this and this should be seen as a last resort.
pub async fn yield_now() {
  yield_now::YieldNow::default().await
}

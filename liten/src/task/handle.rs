use std::{future::Future, pin::Pin};

/// A handle to a spawned task that can be used to join with the task or poll it as a future.
///
/// `TaskHandle<Out>` represents a handle to an asynchronous task that will eventually produce
/// a value of type `Out`. The task continues running in the background until it completes
/// or is dropped.
///
/// # Examples
///
// /// ```rust
// /// use liten::task;
// ///
// /// let handle = task::spawn(async {
// ///     // Some async work
// ///     42
// /// });
// ///
// /// // Join with the task to get the result
// /// let result = handle.join();
// /// assert_eq!(result, 42);
// /// ```
///
/// You can also use it as a future:
///
/// ```rust
/// # #![cfg(feature = "runtime")]
///
/// use liten::task;
///
/// # #[liten::main]
/// # async fn main() {
/// let handle = task::spawn(async {
///     // Some async work
///     "hello".to_string()
/// });
///
/// assert_eq!(handle.await, "hello");
/// # }
/// ```
///
/// # Drop behavior
///
/// When a `TaskHandle` is dropped, the associated task continues running in the background.
/// If you want to ensure the task completes, you should call `join()` before dropping the handle.
pub struct TaskHandle<Out> {
  task: Option<async_task::Task<Out>>,
  be_stopped: bool,
}

impl<O> TaskHandle<O> {
  pub(crate) fn new(t: async_task::Task<O>) -> Self {
    TaskHandle { task: Some(t), be_stopped: false }
  }

  pub fn cancel(&mut self) {
    self.be_stopped = true;
    let task = self.task.take().unwrap();
    drop(task);
  }

  pub fn join(self) -> O {
    crate::block_on(self)
  }
}

impl<D> Drop for TaskHandle<D> {
  fn drop(&mut self) {
    if !self.be_stopped {
      let task = self.task.take().expect("guarranteed to be there.");
      task.detach();
    }
  }
}

impl<Out> Future for TaskHandle<Out> {
  type Output = Out;
  fn poll(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    let mut task = self.task.take().expect("guarranteed to be there");

    let pinned = Pin::new(&mut task);
    let result = pinned.poll(cx);

    self.task.replace(task);

    result
  }
}

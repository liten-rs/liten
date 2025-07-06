pub(crate) mod pool;

use pool::{BlockingPool, Job};

use crate::sync::oneshot;

/// Executes a blocking function on a dedicated thread pool without blocking runtime workers.
///
/// This function is useful when you need to perform CPU-intensive or blocking I/O operations
/// that would otherwise block the async runtime's worker threads. By offloading these operations
/// to a separate thread pool, the runtime can continue processing other async tasks efficiently.
/// This method is as transparent as possible. It takes a function and returns what the function returns.
///
/// # Examples
///
/// ```rust
/// use liten::blocking;
///
/// # async fn example() {
/// let result = blocking::unblock(|| {
///     // CPU-intensive work or blocking I/O
///     std::thread::sleep(std::time::Duration::from_millis(100));
///     42
/// }).await;
/// assert_eq!(result, 42);
/// # }
/// ```
///
/// # Why use this?
///
/// Async runtimes typically have a limited number of worker threads that handle all async tasks.
/// If you perform blocking operations (like file I/O, network calls, or CPU-intensive computations)
/// directly on these worker threads, you can starve other async tasks and reduce overall throughput (espacially on a single-thread runtime).
/// This function ensures that blocking work doesn't interfere with the runtime's ability to process
/// other concurrent tasks efficiently.
pub async fn unblock<T, R>(f: T) -> R
where
  T: FnOnce() -> R + Send + 'static,
  R: 'static + Send,
{
  let (sender, receiver) = oneshot::channel::<R>();
  BlockingPool::get().insert(Job::new(sender, f));
  receiver.await.unwrap()
}

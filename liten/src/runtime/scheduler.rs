use std::future::Future;

use crate::task::Task;

pub(crate) mod multi_threaded;
pub(crate) mod single_threaded;

pub trait Scheduler {
  fn schedule(task: Task) {
    let _ = task;
  }
  fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future;
}

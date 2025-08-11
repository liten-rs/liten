use std::future::Future;
mod waker;

#[cfg(feature = "blocking")]
use crate::blocking::pool::BlockingPool;
use crate::runtime::scheduler::{single_threaded::SingleThreaded, Scheduler};
#[cfg(all(feature = "time", not(loom)))]
use crate::time::TimeDriver;

pub mod scheduler;

#[derive(Default)]
pub struct Runtime<S> {
  scheduler: S,
}

impl Runtime<SingleThreaded> {
  pub fn single_threaded() -> Self {
    Runtime::with_scheduler(SingleThreaded)
  }
}

impl<S> Runtime<S>
where
  S: Scheduler,
{
  pub fn with_scheduler(scheduler: S) -> Self {
    Runtime { scheduler }
  }
  pub fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    // let scheduler = unsafe { &*(self.scheduler) };
    let to_return = self.scheduler.block_on(fut);

    #[cfg(all(feature = "time", not(loom)))]
    TimeDriver::shutdown();
    #[cfg(feature = "blocking")]
    BlockingPool::shutdown();

    to_return
  }
}

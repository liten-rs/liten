use std::future::Future;
mod waker;

#[cfg(feature = "blocking")]
use crate::blocking::pool::BlockingPool;
use crate::runtime::scheduler::{
  /*multi_threaded::Multithreaded,*/ single_threaded::SingleThreaded,
  Scheduler,
};
#[cfg(feature = "time")]
use crate::time::TimeDriver;

pub mod scheduler;

pub struct Runtime<S: scheduler::Scheduler> {
  scheduler: S,
}

impl Runtime<SingleThreaded> {
  pub fn single_threaded() -> Self {
    Runtime::with_scheduler(SingleThreaded)
  }
}

// impl Runtime<Multithreaded> {
//   pub fn multi_threaded() -> Self {
//     Runtime::with_scheduler(Multithreaded::default())
//   }
// }

impl<T> Runtime<T>
where
  T: Scheduler,
{
  pub fn with_scheduler(scheduler: T) -> Self {
    Runtime { scheduler }
  }

  pub fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    let to_return = self.scheduler.block_on(fut);

    #[cfg(feature = "time")]
    TimeDriver::shutdown();
    #[cfg(feature = "blocking")]
    BlockingPool::shutdown();

    to_return
  }
}

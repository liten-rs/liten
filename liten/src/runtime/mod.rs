use std::{
  future::Future,
  sync::OnceLock,
  task::{Context, Poll},
  thread::{self, Thread},
};

#[cfg(feature = "blocking")]
use crate::blocking::pool::BlockingPool;
#[cfg(all(feature = "time", not(loom)))]
use crate::time::TimeDriver;
use crate::{
  future::block_on::park_waker,
  parking,
  runtime::scheduler::{single_threaded::SingleThreaded, Scheduler},
  task::store::TaskStore,
};

pub mod scheduler;

#[derive(Default)]
pub struct Runtime<S> {
  scheduler: S,
}

impl Runtime<SingleThreaded> {
  pub const fn single_threaded() -> Self {
    Runtime::with_scheduler(SingleThreaded)
  }
}

impl<S> Runtime<S>
where
  S: Scheduler,
{
  pub const fn with_scheduler(scheduler: S) -> Self {
    Runtime { scheduler }
  }

  pub fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    let _thread = parking::set_main_thread();
    let mut fut = std::pin::pin!(fut);

    let to_return: F::Output = loop {
      self.scheduler.tick(TaskStore::get().tasks());

      let waker = park_waker(_thread.clone());
      if let Poll::Ready(value) =
        fut.as_mut().poll(&mut Context::from_waker(&waker))
      {
        break value;
      }

      #[cfg(feature = "io")]
      lio::tick();

      parking::park();
    };

    #[cfg(all(feature = "time", not(loom)))]
    TimeDriver::shutdown();
    #[cfg(feature = "blocking")]
    BlockingPool::shutdown();

    to_return
  }
}

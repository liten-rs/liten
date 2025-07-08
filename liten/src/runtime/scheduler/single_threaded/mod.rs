// mod waker;

use std::{
  future::{Future, IntoFuture},
  task::{Context, Poll},
};

// TODO: pub is temp
pub mod waker;

use crate::{
  runtime::scheduler::{waker::create_runtime_waker, Scheduler},
  task::TaskStore,
};

pub struct SingleThreaded;

impl Scheduler for SingleThreaded {
  fn block_on<F, R>(self, fut: F) -> R
  where
    F: IntoFuture<Output = R>,
  {
    let mut fut = std::pin::pin!(fut.into_future());

    let parker = parking::Parker::new();
    let unparker = parker.unparker();

    let waker = create_runtime_waker(unparker.clone());

    if let Poll::Ready(value) =
      fut.as_mut().poll(&mut Context::from_waker(&waker))
    {
      return value;
    }

    loop {
      TaskStore::get().move_cold_to_hot();
      loop {
        match TaskStore::get().task_dequeue() {
          Some(task) => {
            let waker = waker::create_task_waker(unparker.clone(), task.id());
            task.poll(&mut Context::from_waker(&waker));
          }
          None => break,
        };
      }

      let waker = create_runtime_waker(parker.unparker());
      if let Poll::Ready(value) =
        fut.as_mut().poll(&mut Context::from_waker(&waker))
      {
        return value;
      }

      parker.park();
    }
  }
}

// use std::{future::Future, io};
//
// use crate::{
//   context,
//   runtime::{main_executor::GlobalExecutor, scheduler::worker::shared::Shared},
// };
//
// use super::{RuntimeBuilder, trait::SchedulerTrait};
//
// /// Single-threaded scheduler that runs all tasks on the main thread.
// #[derive(Debug)]
// pub struct SingleThreadedScheduler;
//
// impl SchedulerTrait for SingleThreadedScheduler {
//   /// Blocks the current thread until the future completes.
//   ///
//   /// In single-threaded mode, all tasks run on the main thread,
//   /// providing deterministic execution but no parallelism.
//   fn block_on<F, Res>(self, fut: F, _config: RuntimeBuilder) -> Res
//   where
//     F: Future<Output = Res>,
//   {
//     let driver = Driver::new().unwrap();
//     let handle = driver.handle(Shared::new_without_remotes());
//
//     // Single-threaded mode: run everything on the main thread
//     context::runtime_enter(handle, move |_| GlobalExecutor::block_on(fut))
//   }
// }
//
// impl Drop for SingleThreadedScheduler {
//   fn drop(&mut self) {
//     #[cfg(feature = "blocking")]
//     crate::blocking::pool::BlockingPool::shutdown();
//
//     #[cfg(feature = "time")]
//     crate::time::TimeDriver::shutdown();
//   }
// }
//
// #[derive(Debug, Clone)]
// pub struct Handle {
//   pub shared: crate::loom::sync::Arc<Shared>,
// }
//
// #[cfg(test)]
// static_assertions::assert_impl_one!(Handle: Send);
//
// impl Handle {
//   pub fn state(&self) -> &Shared {
//     self.shared.as_ref()
//   }
// }

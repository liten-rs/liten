pub(crate) mod scheduler;
mod waker;

use std::{future::Future, sync::Arc};

use scheduler::{
  worker::{Shared, WorkersBuilder},
  Scheduler,
};

use crate::{context, io_loop};

pub struct Runtime {
  scheduler: Scheduler,
  handle: Arc<scheduler::Handle>,
  driver: scheduler::Driver,
}

impl Runtime {
  pub fn new() -> Self {
    let (io_driver, io_handle) = io_loop::Driver::new().unwrap();
    let shared = Shared::new(2);
    let handle = scheduler::Handle::new(io_handle, shared.clone());
    Runtime {
      scheduler: Scheduler,
      driver: scheduler::Driver { io: io_driver },
      handle: Arc::new(handle),
    }
  }

  pub fn block_on<F, Res>(&mut self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    let workers = WorkersBuilder::from(self.handle.clone());

    context::runtime_enter(self.handle.clone(), |_| {
      workers.launch();
      self.scheduler.block_on(fut)
    })

    //self.scheduler.block_on(handle, move |cx| {
    //
    //})
    //self.scheduler.block_on(.handle, |ctx| loop {
    //  let context: std::task::Context;
    //  if let Poll::Ready(output) = main_fut.as_mut().poll(&mut context) {
    //    return output;
    //  }
    //})
    //self.scheduler.block_on(self.handle, |ctx| loop {})
    //// TODO: remove everything and implement better shit.
    //let (runtime_sender, runtime_receiver) = channel::unbounded();
    //let waker = Arc::new(RuntimeWaker::new(runtime_sender)).into();
    //let mut main_fut_context = StdContext::from_waker(&waker);
    //
    //let main_fut = Box::pin(fut);
    //
    //let mut pinned = std::pin::pin!(main_fut);
    //// Starts the poll so that the waker gets a change to send from the receiver.
    //if let Poll::Ready(value) = pinned.as_mut().poll(&mut main_fut_context) {
    //  return value;
    //};
    //loop {
    //  if runtime_receiver.try_recv().is_ok() {
    //    if let Poll::Ready(value) = pinned.as_mut().poll(&mut main_fut_context)
    //    {
    //      return value;
    //    };
    //  }
    //  // Fill the newest tasks onto the task queue.
    //  self.scheduler.tick();
    //}
  }
}

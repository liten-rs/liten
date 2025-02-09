pub(crate) mod scheduler;
mod waker;

use std::{
  future::Future,
  sync::Arc,
  task::{Context as StdContext, Poll, Wake},
};

use crossbeam::channel::{self, Sender};
use scheduler::Scheduler;
use waker::{LitenWaker, RuntimeWaker};

use crate::{
  context::{self, ContextDropper},
  io_loop,
  task::Task,
  taskqueue::TaskQueue,
};

pub struct Runtime {
  scheduler: Scheduler,

  handle: Arc<scheduler::Handle>,
}

impl Runtime {
  pub fn new() -> Self {
    let (driver, handle) = io_loop::Driver::new();
    let handle = Arc::new(scheduler::Handle {
      io: handle,
      shared: scheduler::worker::Shared {
        inject: crossbeam::deque::Injector::new(),
        remotes: Box::new([]),
      },
    });
    Runtime { scheduler: Scheduler, handle }
  }

  pub fn block_on<F, Res>(&mut self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    // TODO: remove everything and implement better shit.
    let (runtime_sender, runtime_receiver) = channel::unbounded();
    let waker = Arc::new(RuntimeWaker::new(runtime_sender)).into();
    let mut main_fut_context = StdContext::from_waker(&waker);

    let main_fut = Box::pin(fut);

    let mut pinned = std::pin::pin!(main_fut);
    // Starts the poll so that the waker gets a change to send from the receiver.
    if let Poll::Ready(value) = pinned.as_mut().poll(&mut main_fut_context) {
      return value;
    };
    loop {
      if runtime_receiver.try_recv().is_ok() {
        if let Poll::Ready(value) = pinned.as_mut().poll(&mut main_fut_context)
        {
          return value;
        };
      }
      // Fill the newest tasks onto the task queue.
      self.scheduler.tick();
    }
  }
}

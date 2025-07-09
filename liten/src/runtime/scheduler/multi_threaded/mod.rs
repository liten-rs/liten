mod worker;

use std::{future::Future, thread::available_parallelism};

use crate::{
  loom::sync::Arc,
  runtime::{scheduler::Scheduler, Runtime},
};
use worker::Workers;

#[derive(Debug)]
pub struct Multithreaded {
  threads: u16,
  work_stealing: bool,
}

impl Multithreaded {
  pub fn work_stealing(&self) -> bool {
    self.work_stealing
  }

  pub fn threads(&self) -> u16 {
    self.threads
  }
}

impl Default for Multithreaded {
  fn default() -> Self {
    Self {
      threads: available_parallelism().unwrap().get() as u16,
      work_stealing: false,
    }
  }
}

impl Scheduler for Multithreaded {
  fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    let workers = Workers::new(Arc::new(self));

    let mut shutdown = workers.as_shutdown_workers();
    shutdown.fill_handle(workers.launch());

    let return_type = crate::future::block_on(fut);

    shutdown.shutdown();
    return_type
  }
}

impl Runtime<Multithreaded> {
  pub fn threads(mut self, threads: u16) -> Self {
    self.scheduler.threads = threads;
    self
  }

  pub fn work_stealing(mut self, enabled: bool) -> Self {
    assert!(!enabled, "Work stealing does not work currently :(");
    self.scheduler.work_stealing = enabled;
    self
  }
}

// #[derive(Debug, Clone)]
// pub struct Handle {
//   // pub io: events::Handle,
//   pub shared: Arc<Shared>,
// }
//
// #[cfg(test)]
// static_assertions::assert_impl_one!(Handle: Send);

// pub struct Driver {
//   // io: events::Driver,
// }
//
// impl Driver {
//   pub fn new() -> io::Result<Self> {
//     Ok(Self { /*io: events::Driver::new()?*/ })
//   }
//   pub fn handle(&self, shared: Shared) -> Handle {
//     Handle { /*io: self.io.handle(),*/ shared: Arc::new(shared) }
//   }
// }

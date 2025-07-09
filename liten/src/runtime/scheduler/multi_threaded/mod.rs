mod worker;

use std::{future::Future, thread::available_parallelism};

use crate::{
  loom::sync::Arc,
  runtime::{scheduler::Scheduler, Runtime},
  task::TaskStore,
};
use worker::Workers;

#[derive(Debug)]
pub struct Multithreaded {
  threads: u16,
}

impl Multithreaded {
  pub fn threads(&self) -> u16 {
    self.threads
  }
}

impl Default for Multithreaded {
  fn default() -> Self {
    let threads = available_parallelism().unwrap().get() as u16;
    Self { threads }
  }
}

impl Scheduler for Multithreaded {
  fn schedule(task: crate::task::Task) {
    TaskStore::get().task_enqueue(task);
  }
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
}

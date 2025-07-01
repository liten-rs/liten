pub mod worker;

use std::{future::Future, io};

use crate::{
  blocking::pool::BlockingPool,
  context, events,
  loom::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
  runtime::{main_executor::GlobalExecutor, scheduler::worker::shared::Shared},
  time::TimeDriver,
};
use worker::Workers;

use super::RuntimeBuilder;

#[derive(Debug)]
pub struct Scheduler;

impl Scheduler {
  pub fn block_on<F, Res>(self, fut: F, config: RuntimeBuilder) -> Res
  where
    F: Future<Output = Res>,
  {
    let driver = Driver::new().unwrap();

    let handle = driver.handle(Shared::new_without_remotes());

    let workers = Workers::new(Arc::new(config));

    handle.shared.fill_remotes(&workers);

    let mut shutdown = workers.as_shutdown_workers();

    shutdown.fill_handle(workers.launch(handle.clone()));

    // NOTE: Has to be over the mio join handle.
    let shutdown_waker = handle.io().shutdown_waker();

    let return_type =
      context::runtime_enter(handle, move |_| GlobalExecutor::block_on(fut));

    shutdown.shutdown();

    shutdown_waker.wake().expect("noo :(");

    BlockingPool::shutdown();
    TimeDriver::shutdown();

    return_type
  }
}

#[derive(Debug, Clone)]
pub struct Handle {
  pub io: events::Handle,
  pub shared: Arc<Shared>,
  // TODO: maybe move this into some sort of WorkerHandle?
  current_task_id: Arc<AtomicUsize>,
}

#[cfg(test)]
static_assertions::assert_impl_one!(Handle: Send);

impl Handle {
  pub fn state(&self) -> &Shared {
    self.shared.as_ref()
  }

  pub fn io(&self) -> &events::Handle {
    &self.io
  }

  /// Returns the previous value
  pub fn task_id_inc(&self) -> usize {
    self.current_task_id.fetch_add(1, Ordering::SeqCst)
  }
}

pub struct Driver {
  io: events::Driver,
}

impl Driver {
  pub fn new() -> io::Result<Self> {
    Ok(Self { io: events::Driver::new()? })
  }
  pub fn handle(&self, shared: Shared) -> Handle {
    Handle {
      io: self.io.handle(),
      shared: Arc::new(shared),
      current_task_id: Arc::new(AtomicUsize::new(0)),
    }
  }
}

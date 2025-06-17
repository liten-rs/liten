pub mod worker;

use std::{future::Future, io};

use crate::{
  context, events,
  loom::{
    sync::{
      atomic::{AtomicUsize, Ordering},
      Arc,
    },
    thread,
  },
  runtime::{main_executor::GlobalExecutor, scheduler::worker::shared::Shared},
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
    let mut driver = Driver::new().unwrap();

    let handle = driver.handle(Shared::new_without_remotes());

    let workers = Workers::new(config.get_num_workers());

    handle.shared.fill_remotes(&workers);

    let mut shutdown = workers.as_shutdown_workers();

    shutdown.fill_handle(workers.launch(handle.clone()));

    let span = tracing::trace_span!("liten runtime");
    let _span = span.enter();

    // NOTE: Has to be over the mio join handle.
    let shutdown_waker = handle.io().shutdown_waker();

    let join_handle = thread::spawn(move || loop {
      if driver.io.turn() {
        tracing::trace!("shutting down driver thread");
        break;
      }
    });

    let return_type =
      context::runtime_enter(handle, move |_| GlobalExecutor::block_on(fut));

    shutdown.shutdown();

    shutdown_waker.wake().expect("noo :(");
    join_handle.join().unwrap();

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

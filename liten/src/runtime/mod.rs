mod main_executor;
pub(crate) mod scheduler;
mod waker;

use scheduler::{worker::Shared, Scheduler};
use std::{future::Future, sync::Arc};

use crate::events;

pub struct Runtime {
  scheduler: Scheduler,
}

impl Runtime {
  pub fn new() -> Self {
    Runtime { scheduler: Scheduler }
  }

  pub fn block_on<F, Res>(self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    let (io_driver, io_handle) = events::Driver::new().unwrap();

    let driver = scheduler::Driver { io: io_driver };
    let handle = Arc::new(scheduler::Handle::new(io_handle));
    let (workers, shared, shutdown) = Shared::new_parts(8, handle.clone());

    handle.set_handle(shared);
    self.scheduler.block_on(handle, driver, workers, shutdown, fut)
  }
}

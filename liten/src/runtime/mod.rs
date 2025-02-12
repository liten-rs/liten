mod main_executor;
pub(crate) mod scheduler;
mod waker;

use scheduler::Scheduler;
use std::future::Future;

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
    //let (io_driver, io_handle) = events::Driver::new().unwrap();
    //
    //let driver = scheduler::Driver { io: io_driver };
    //let handle = Arc::new(scheduler::Handle::new(io_handle));
    //
    //let cpus = std::thread::available_parallelism().unwrap();
    //
    //let workers = Workers::new(cpus, handle.clone());
    //
    //// TODO: Create workers
    //
    ////let (workers, shared, shutdown) = Shared::new_parts(cpus, handle.clone());
    //
    //let shared = Shared::from_workers(&workers);
    //
    //handle.set_handle(shared);
    self.scheduler.block_on(fut)
  }
}

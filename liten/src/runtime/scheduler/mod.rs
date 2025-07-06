pub mod worker;

use std::{future::Future, io};

use crate::{
  context,
  loom::sync::Arc,
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
    let driver = Driver::new().unwrap();

    let handle = driver.handle(Shared::new_without_remotes());

    let workers = Workers::new(Arc::new(config));

    handle.shared.fill_remotes(&workers);

    let mut shutdown = workers.as_shutdown_workers();

    shutdown.fill_handle(workers.launch(handle.clone()));

    // NOTE: Has to be over the mio join handle.
    // let shutdown_waker = handle.io().shutdown_waker();

    let return_type =
      context::runtime_enter(handle, move |_| GlobalExecutor::block_on(fut));

    shutdown.shutdown();

    return_type
  }
}

impl Drop for Scheduler {
  fn drop(&mut self) {
    #[cfg(feature = "blocking")]
    crate::blocking::pool::BlockingPool::shutdown();

    #[cfg(feature = "time")]
    crate::time::TimeDriver::shutdown();
  }
}

#[derive(Debug, Clone)]
pub struct Handle {
  // pub io: events::Handle,
  pub shared: Arc<Shared>,
}

#[cfg(test)]
static_assertions::assert_impl_one!(Handle: Send);

impl Handle {
  pub fn state(&self) -> &Shared {
    self.shared.as_ref()
  }

  // pub fn io(&self) -> &events::Handle {
  //   &self.io
  // }
}

pub struct Driver {
  // io: events::Driver,
}

impl Driver {
  pub fn new() -> io::Result<Self> {
    Ok(Self { /*io: events::Driver::new()?*/ })
  }
  pub fn handle(&self, shared: Shared) -> Handle {
    Handle { /*io: self.io.handle(),*/ shared: Arc::new(shared) }
  }
}

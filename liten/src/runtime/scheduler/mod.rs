pub mod worker;
use crate::runtime::scheduler::worker::shared::Shared;

use std::{future::Future, sync::OnceLock};

use crate::loom::sync::{
  atomic::{AtomicBool, AtomicUsize, Ordering},
  Arc,
};

use super::{super::events, main_executor::GlobalExecutor};
use crate::context;
use worker::Workers;

#[derive(Debug)]
pub struct Scheduler;

impl Scheduler {
  pub fn block_on<F, Res>(self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    let (io_driver, io_handle) = events::Driver::new().unwrap();

    let mut driver = Driver { io: io_driver };
    let handle = Arc::new(Handle::without_shared(io_handle));

    let cpus = std::thread::available_parallelism().unwrap();

    let workers = Workers::new(cpus, handle.clone());

    let shared = Shared::from_workers(&workers);
    handle.set_handle(shared);

    let mut shutdown = workers.as_shutdown_workers();

    let handles = workers.launch(handle.clone());
    shutdown.fill_handle(handles);

    let span = tracing::trace_span!("liten runtime");
    let _span = span.enter();

    // NOTE: Has to be over the mio join handle.
    let mio_waker = handle.io().mio_waker();

    let thread_handle = handle.clone();

    let join_handle = std::thread::spawn(move || loop {
      if driver.io.turn(thread_handle.io()) {
        break;
      }
    });

    let return_type =
      context::runtime_enter(handle, move |_| GlobalExecutor::block_on(fut));

    shutdown.shutdown();

    mio_waker.wake().expect("noo :(");
    join_handle.join().unwrap();

    return_type
  }
}

pub struct Handle {
  pub io: events::Handle,
  pub shared: OnceLock<Arc<Shared>>,

  current_task_id: AtomicUsize,
  has_exited: AtomicBool,
}

impl Handle {
  pub fn state(&self) -> &Shared {
    self.shared.get().expect("state not set")
  }

  pub fn io(&self) -> &events::Handle {
    &self.io
  }

  pub fn without_shared(io: events::Handle) -> Handle {
    Handle {
      io,
      shared: OnceLock::new(),
      has_exited: AtomicBool::new(false),
      current_task_id: AtomicUsize::new(0),
    }
  }
  /// Returns the previous value
  pub fn task_id_inc(&self) -> usize {
    self.current_task_id.fetch_add(1, Ordering::SeqCst)
  }

  pub fn has_entered(&self) -> bool {
    !self.has_exited.load(std::sync::atomic::Ordering::SeqCst)
  }
  pub fn exit(&self) {
    if !self.has_exited.swap(true, std::sync::atomic::Ordering::SeqCst) {
      // This can happen if a worker is started on the main thread.
    }
  }

  pub fn set_handle(&self, shared: Arc<Shared>) {
    if self.shared.set(shared).is_err() {
      panic!("what the fuuuuck");
    }
  }
}

pub struct Driver {
  pub io: events::Driver,
}

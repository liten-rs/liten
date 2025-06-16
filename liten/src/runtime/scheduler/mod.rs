pub mod worker;

use std::{future::Future, sync::OnceLock};

use crate::{
  context, events,
  loom::{
    sync::{
      atomic::{AtomicBool, AtomicUsize, Ordering},
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
    let events_driver = events::Driver::new().unwrap();
    let events_handle = events_driver.handle().unwrap();

    let mut driver = Driver { io: events_driver };
    let handle = Arc::new(Handle::without_shared(events_handle));

    let workers = Workers::new(config.get_num_workers(), handle.clone());

    handle.set_handle(Shared::from_workers(&workers));

    dbg!(&handle);

    let mut shutdown = workers.as_shutdown_workers();

    let handles = workers.launch(handle.clone());
    shutdown.fill_handle(handles);

    let span = tracing::trace_span!("liten runtime");
    let _span = span.enter();

    // NOTE: Has to be over the mio join handle.
    let mio_waker = handle.io().mio_waker();

    let thread_handle = handle.clone();

    let join_handle = thread::spawn(move || loop {
      if driver.io.turn(thread_handle.io()) {
        tracing::trace!("shutting down driver thread");
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

#[derive(Debug, Clone)]
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
    !self.has_exited.load(Ordering::SeqCst)
  }
  pub fn exit(&self) {
    if !self.has_exited.swap(true, Ordering::SeqCst) {
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

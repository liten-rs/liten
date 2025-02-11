pub mod worker;

use std::{
  future::Future,
  sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, OnceLock,
  },
};

use worker::{worker::Worker, ShutdownWorkers, Workers};

use crate::context;

use super::{super::events, main_executor::GlobalExecutor};

#[derive(Debug)]
pub struct Scheduler;

impl Scheduler {
  pub fn block_on<F, Res>(
    self,
    handle: Arc<Handle>,
    mut driver: Driver,
    workers: Vec<Worker>,
    mut shutdown: ShutdownWorkers,
    fut: F,
  ) -> Res
  where
    F: Future<Output = Res>,
  {
    let span = tracing::trace_span!("liten runtime");
    let _span = span.enter();

    let workers = Workers::from(workers);
    for (index, handle) in
      workers.launch(handle.clone()).into_iter().enumerate()
    {
      shutdown.set_handle(index, handle)
    }

    // DONT TOUCH: mio waker must be defined before driver.io.turn(...)
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
  pub shared: OnceLock<Arc<worker::Shared>>,

  current_task_id: AtomicUsize,
  has_exited: AtomicBool,
}

impl Handle {
  pub fn new(io: events::Handle) -> Handle {
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

  pub fn set_handle(&self, shared: Arc<worker::Shared>) {
    if self.shared.set(shared).is_err() {
      panic!("what the fuuuuck");
    }
  }
}

pub struct Driver {
  pub io: events::Driver,
}

impl Handle {
  pub fn state(&self) -> &worker::Shared {
    &self.shared.get().expect("state not set")
  }
}

impl Handle {
  pub fn io(&self) -> &events::Handle {
    &self.io
  }
}

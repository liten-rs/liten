use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread::{self, JoinHandle},
};

#[cfg(feature = "blocking")]
use crate::blocking::pool::BlockingPool;
#[cfg(all(feature = "time", not(loom)))]
use crate::time::TimeDriver;
use crate::{
  runtime::{
    scheduler::{Scheduler, SingleThreaded},
    PARKER,
  },
  task::store::TaskStore,
};

pub struct CoroHandle {
  // scheduler: SingleThreaded,
  handle: Option<JoinHandle<()>>,
  state: Arc<CoroState>,
}

struct CoroState {
  shutdown_singal: AtomicBool,
}

impl Default for CoroState {
  fn default() -> Self {
    Self { shutdown_singal: AtomicBool::new(false) }
  }
}

impl Drop for CoroHandle {
  fn drop(&mut self) {
    self.state.shutdown_singal.store(true, Ordering::Relaxed);

    PARKER.get().unwrap().unpark();

    self
      .handle
      .take()
      .expect("handle taken twice")
      .join()
      .expect("thread panicked");
  }
}

pub fn init() -> CoroHandle {
  init_with_schedule(SingleThreaded)
}

pub fn init_with_schedule<S: Scheduler + Send + 'static>(
  scheduler: S,
) -> CoroHandle {
  let state = Arc::new(CoroState::default());
  let handle = thread::spawn({
    let state = state.clone();
    move || main_thread(scheduler, state)
  });

  CoroHandle { handle: Some(handle), state }
}

fn main_thread<S: Scheduler>(scheduler: S, state: Arc<CoroState>) {
  let _thread = thread::current();
  PARKER.set(_thread).unwrap();
  let store = TaskStore::get();
  loop {
    if state.shutdown_singal.load(Ordering::Acquire) {
      break;
    }

    scheduler.tick(store.tasks());

    #[cfg(all(feature = "io", not(miri)))]
    lio::tick();

    thread::park();
  }
  #[cfg(all(feature = "time", not(loom)))]
  TimeDriver::shutdown();
  #[cfg(feature = "blocking")]
  BlockingPool::shutdown();
}

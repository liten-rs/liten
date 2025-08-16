use std::{
  future::Future,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
  },
  thread::{self, JoinHandle},
};

#[cfg(feature = "blocking")]
use crate::blocking::pool::BlockingPool;
#[cfg(all(feature = "time", not(loom)))]
use crate::time::TimeDriver;
use crate::{
  runtime::scheduler::{Scheduler, SingleThreaded},
  task::{store::TaskStore, TaskHandle},
};

struct CoroHandle {
  handle: Mutex<Option<JoinHandle<()>>>,
  state: Arc<CoroState>,
}

struct CoroState {
  shutdown_singal: AtomicBool,
  queue: TaskStore,
}
impl Default for CoroState {
  fn default() -> Self {
    Self { shutdown_singal: AtomicBool::new(false), queue: TaskStore::new() }
  }
}

impl CoroHandle {
  fn with_scheduler<S: Scheduler + Send + 'static>(scheduler: S) -> Self {
    let state = Arc::new(CoroState::default());
    let handle = thread::spawn({
      let state = state.clone();
      move || Self::bg_thread(scheduler, state)
    });

    CoroHandle { handle: Mutex::new(Some(handle)), state }
  }
  fn bg_thread<S: Scheduler>(scheduler: S, state: Arc<CoroState>) {
    loop {
      if state.shutdown_singal.load(Ordering::Acquire) {
        break;
      }

      scheduler.tick(state.queue.tasks());

      #[cfg(all(feature = "io", not(miri)))]
      lio::tick();

      thread::park();
    }

    #[cfg(all(feature = "time", not(loom)))]
    TimeDriver::shutdown();
    #[cfg(feature = "blocking")]
    BlockingPool::shutdown();
  }
}

std::thread_local! {
  static CORO: OnceLock<CoroHandle> = OnceLock::new();
}

pub fn init_with_scheduler<S: Scheduler + Send + 'static>(scheduler: S) {
  CORO.with(|thing| {
    if let Err(_) = thing.set(CoroHandle::with_scheduler(scheduler)) {
      // TODO: good with panic?
      unreachable!();
    }
  })
}

pub fn shutdown() {
  CORO.with(|coro| {
    let this = coro.get().unwrap();
    this.state.shutdown_singal.store(true, Ordering::SeqCst);

    let mut _lock = this.handle.lock().unwrap();
    let handle = _lock.take().expect("handle taken twice");
    // Cannot have mutex locked after waking worker thread.
    drop(_lock);
    handle.thread().unpark();

    handle.join().expect("thread panicked");
  });
}

pub fn go<F>(f: F) -> TaskHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send,
{
  let (runnable, task) = async_task::spawn(f, |runnable| {
    CORO.with(|thing| {
      let handle =
        thing.get_or_init(|| CoroHandle::with_scheduler(SingleThreaded));

      handle.state.queue.task_enqueue(runnable);
      let _lock = handle.handle.lock().unwrap();
      let handle = _lock.as_ref().unwrap();
      handle.thread().unpark();
    })
  });
  runnable.schedule();

  TaskHandle::new(task)
}

use std::sync::{atomic::AtomicBool, OnceLock};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use private::JobRun;

use crate::loom::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};
use crate::loom::thread;

pub(crate) struct BlockingPool {
  queue: (Sender<Box<dyn JobRun>>, Receiver<Box<dyn JobRun>>),
  thread_state: Arc<ThreadState>,
}

#[derive(Debug)]
struct ThreadState {
  threads_running: AtomicUsize,
  threads_busy: AtomicUsize,
  max_threads: usize,
  shutdown_signal: AtomicBool,
}

struct ThreadPanicGuard<'a>(&'a BlockingPool, bool);

impl<'a> ThreadPanicGuard<'a> {
  pub fn new(pool: &'a BlockingPool) -> Self {
    Self(pool, true)
  }

  pub fn disactivate(&mut self) {
    self.1 = false;
  }
}

impl Drop for ThreadPanicGuard<'_> {
  fn drop(&mut self) {
    if self.1 && std::thread::panicking() {
      self.0.thread_state.threads_running.fetch_sub(1, Ordering::AcqRel);
      self.0.thread_state.threads_busy.fetch_sub(1, Ordering::AcqRel);
    }
  }
}

impl BlockingPool {
  pub(super) fn get() -> &'static BlockingPool {
    static BLOCKING_POOL: OnceLock<BlockingPool> = OnceLock::new();
    let blocking = BLOCKING_POOL.get_or_init(|| BlockingPool {
      thread_state: Arc::new(ThreadState {
        max_threads: 500,
        threads_busy: AtomicUsize::new(0),
        threads_running: AtomicUsize::new(0),
        shutdown_signal: AtomicBool::new(false),
      }),
      queue: crossbeam_channel::bounded(500),
    });

    blocking
  }

  pub(crate) fn shutdown() {
    let thing = Self::get();
    thing.thread_state.shutdown_signal.store(true, Ordering::SeqCst);

    while thing.thread_state.threads_running.load(Ordering::SeqCst) > 0 {
      std::thread::yield_now();
    }
  }

  fn add_thread(&'static self) {
    if self.thread_state.shutdown_signal.load(Ordering::SeqCst) {
      return;
    }

    let threads_busy = self.thread_state.threads_busy.load(Ordering::SeqCst);
    let threads_running =
      self.thread_state.threads_running.load(Ordering::SeqCst);
    let threads_max = self.thread_state.max_threads;

    if threads_running == threads_max {
      return;
    }

    if threads_busy != threads_running {
      return;
    }

    thread::spawn(|| self.main_loop());
  }

  pub(super) fn main_loop(&self) {
    println!("starting thread... {:?}", self.thread_state);
    self.thread_state.threads_running.fetch_add(1, Ordering::SeqCst);

    let mut _guard = ThreadPanicGuard::new(self);
    // TODO
    loop {
      match self.queue.1.recv_timeout(Duration::from_secs(5)) {
        Ok(mut job) => {
          self.thread_state.threads_busy.fetch_add(1, Ordering::SeqCst);
          job.run();
          self.thread_state.threads_busy.fetch_sub(1, Ordering::SeqCst);

          if self.thread_state.shutdown_signal.load(Ordering::SeqCst) {
            break;
          }
        }
        Err(RecvTimeoutError::Timeout) => break,
        Err(RecvTimeoutError::Disconnected) => unreachable!(),
      };
    }
    _guard.disactivate();
    self.thread_state.threads_running.fetch_sub(1, Ordering::SeqCst);

    println!("shutting thread... {:?}", self.thread_state);
  }

  pub(super) fn insert<R, F>(&'static self, job: Job<R, F>)
  where
    F: FnOnce() -> R + Send + 'static,
    R: 'static + Send,
  {
    if !self.thread_state.shutdown_signal.load(Ordering::SeqCst) {
      self.add_thread();
      self.queue.0.send(Box::new(job));
    }
  }
}

pub(super) struct Job<R: Send, F: FnOnce() -> R + Send> {
  _fn: Option<F>,
  sender: Option<crate::sync::oneshot::Sender<R>>,
}

impl<R: Send, F: FnOnce() -> R + Send> Job<R, F> {
  pub(super) fn new(sender: crate::sync::oneshot::Sender<R>, _fn: F) -> Self {
    Self { _fn: Some(_fn), sender: Some(sender) }
  }
}

mod private {

  use super::Job;

  // Generic type erasing
  pub(super) trait JobRun: Send // where
  //   &Self: UnwindRefSafe,
  {
    fn run(&mut self);
  }

  impl<R, F> JobRun for Job<R, F>
  where
    F: FnOnce() -> R + Send,
    R: Send,
  {
    fn run(&mut self) {
      let out = (self._fn.take().expect(""))();
      self.sender.take().expect("").send(out).unwrap();
    }
  }
}

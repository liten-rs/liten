use std::{collections::VecDeque, sync::OnceLock};

use private::JobRun;

use crate::loom::sync::{Arc, Mutex};

use crate::loom::sync::atomic::{AtomicUsize, Ordering};
use crate::loom::thread;
use crate::sync::oneshot::Sender;

pub(super) struct BlockingPool {
  threads: AtomicUsize,
  queue: Arc<Mutex<VecDeque<Box<dyn JobRun>>>>,
}

const MAX_THREADS: usize = 500;

impl BlockingPool {
  pub(super) fn get() -> &'static BlockingPool {
    static BLOCKING_POOL: OnceLock<BlockingPool> = OnceLock::new();

    let blocking = BLOCKING_POOL.get_or_init(|| BlockingPool {
      threads: AtomicUsize::new(0),
      queue: Arc::new(Mutex::new(VecDeque::new())),
    });

    blocking
  }

  fn add_thread(&'static self) {
    if self.threads.load(Ordering::SeqCst) < MAX_THREADS {
      thread::spawn(|| self.main_loop());
    }
  }

  pub(super) fn main_loop(&self) {
    // TODO
    self.threads.fetch_add(1, Ordering::SeqCst);
    loop {
      let mut lock = self.queue.lock().unwrap();

      let Some(mut job) = lock.pop_front() else {
        break;
      };

      job.run();
    }
    self.threads.fetch_sub(1, Ordering::SeqCst);
  }

  pub(super) fn insert<R, F>(&'static self, job: Job<R, F>)
  where
    F: FnOnce() -> R + Send + 'static,
    R: 'static + Send,
  {
    let mut lock = self.queue.lock().unwrap();

    lock.push_back(Box::new(job));
    self.add_thread();
  }
}

pub(super) struct Job<R: Send, F: FnOnce() -> R + Send> {
  _fn: Option<F>,
  sender: Option<Sender<R>>,
}

impl<R: Send, F: FnOnce() -> R + Send> Job<R, F> {
  pub(super) fn new(sender: Sender<R>, _fn: F) -> Self {
    Self { _fn: Some(_fn), sender: Some(sender) }
  }
}

mod private {
  use super::Job;

  // Generic type erasing
  pub(super) trait JobRun: Send {
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

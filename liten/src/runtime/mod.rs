use std::{
  cell::{Cell, RefCell},
  future::Future,
  marker::PhantomData,
  task::{Context, Poll},
};

use crate::{
  loom::{sync::Arc, thread},
  runtime::scheduler::{single_threaded::SingleThreaded, Scheduler},
  task::{self, store::TaskStore},
};

pub mod scheduler;

// mod parking;
pub(crate) mod waker;

std::thread_local! {
  static THREAD_RUNTIME: RefCell<Option<RuntimeHandle>> = RefCell::new(None);
}

pub struct RuntimeHandle {
  tasks: Arc<TaskStore>,

  // Make non-Send
  _p: PhantomData<Cell<()>>,
}

impl RuntimeHandle {
  pub(crate) fn with<F, R>(f: F) -> R
  where
    F: FnOnce(&RuntimeHandle) -> R,
  {
    THREAD_RUNTIME.with_borrow(|handle| f(handle.as_ref().unwrap()))
  }

  pub(crate) fn spawn<F>(&self, fut: F) -> task::TaskHandle<F::Output>
  where
    F: Future + 'static,
    F::Output: 'static,
  {
    let task_store = self.tasks.clone();
    let scheduler = move |runnable| {
      task_store.task_enqueue(runnable);
    };

    let (runnable, task) =
      unsafe { async_task::spawn_unchecked(fut, scheduler) };
    runnable.schedule();

    task::TaskHandle::new(task)
  }
}

pub struct Runtime {
  scheduler: Box<dyn Scheduler>,
  tasks: Arc<TaskStore>,

  // Make non-Send
  _p: PhantomData<Cell<()>>,
}

impl Default for Runtime {
  fn default() -> Self {
    Self::with_scheduler(SingleThreaded)
  }
}

impl Drop for Runtime {
  fn drop(&mut self) {
    eprintln!("running drop");
    let _ = THREAD_RUNTIME.replace(None);
  }
}

impl Runtime {
  pub fn single_threaded() -> Self {
    Runtime::with_scheduler(SingleThreaded)
  }
}

impl Runtime {
  pub fn with_scheduler<S>(scheduler: S) -> Self
  where
    S: Scheduler + 'static,
  {
    let task_store = Arc::new(TaskStore::new());

    THREAD_RUNTIME
      .set(Some(RuntimeHandle { tasks: task_store.clone(), _p: PhantomData }));

    Runtime {
      scheduler: Box::new(scheduler),
      tasks: task_store,
      _p: PhantomData,
    }
  }

  pub fn spawn<F>(&self, fut: F) -> task::TaskHandle<F::Output>
  where
    F: Future + 'static,
    F::Output: 'static,
  {
    RuntimeHandle::with(|handle| handle.spawn(fut))
  }

  pub fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    let mut fut = std::pin::pin!(fut);

    let res = loop {
      let waker = waker::park_waker(thread::current());
      if let Poll::Ready(value) =
        fut.as_mut().poll(&mut Context::from_waker(&waker))
      {
        break value;
      }

      while let Some(runnable) = self.tasks.pop() {
        self.scheduler.schedule(runnable);
      }

      thread::park();
    };

    res
  }
}

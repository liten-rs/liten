use std::sync::{Arc, Mutex, MutexGuard};

use crossbeam::queue::SegQueue;

use crate::{enter::Enter, task::Task};

pub struct Context {
  _enter: Enter,
  task_queue: SegQueue<Arc<Task>>,
}

static_assertions::assert_impl_all!(Context: Send);

static CONTEXT: Mutex<Context> = Mutex::new(Context::new());

impl Context {
  const fn new() -> Context {
    Context { _enter: Enter::NotEntered, task_queue: SegQueue::new() }
  }

  pub fn push_task(&mut self, task: Arc<Task>) {
    self.task_queue.push(task)
  }

  pub fn pop_task(&mut self) -> Option<Arc<Task>> {
    self.task_queue.pop()
  }
}

pub fn get_context_mut() -> MutexGuard<'static, Context> {
  let mut _lock = CONTEXT.lock().unwrap();
  _lock
}

pub fn enter() -> ContextDropper {
  get_context_mut()._enter = Enter::Entered;
  ContextDropper
}

pub struct ContextDropper;

impl Drop for ContextDropper {
  fn drop(&mut self) {
    get_context_mut()._enter = Enter::NotEntered;
  }
}

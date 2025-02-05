use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};

use crossbeam::{atomic::AtomicCell, channel::Sender};
use once_cell::sync::OnceCell;

use crate::task::Task;

static CONTEXT: Context = Context::new();
pub struct Context {
  current_task_id: AtomicCell<usize>,
  current_reactor_id: AtomicCell<usize>,
  has_entered: AtomicBool,
  sender: OnceCell<Sender<Arc<Task>>>,
}

static_assertions::assert_impl_all!(Context: Send);

impl Context {
  const fn new() -> Context {
    Context {
      has_entered: AtomicBool::new(false),
      sender: OnceCell::new(),
      current_task_id: AtomicCell::new(0),
      current_reactor_id: AtomicCell::new(0),
    }
  }

  /// Returns the previous value
  pub fn task_id_inc(&self) -> usize {
    self.current_task_id.fetch_add(1)
  }

  pub fn mio_token_id_inc(&self) -> usize {
    self.current_reactor_id.fetch_add(1)
  }

  pub fn push_task(&self, task: Arc<Task>) {
    self.sender.get().unwrap().send(task).unwrap();
  }
}

pub fn get_context() -> &'static Context {
  &CONTEXT
}

pub fn runtime_enter(sender: Sender<Arc<Task>>) -> ContextDropper {
  if CONTEXT.has_entered.load(Ordering::Relaxed) {
    panic!("crate 'liten' user error: can't nest 'liten::runtime::Runtime'");
  }

  CONTEXT.has_entered.store(true, Ordering::Relaxed);
  let ctx = get_context();
  ctx.sender.set(sender).unwrap();
  ctx.has_entered.store(true, Ordering::Relaxed);

  ContextDropper
}

pub struct ContextDropper;

impl Drop for ContextDropper {
  fn drop(&mut self) {
    get_context().has_entered.store(false, Ordering::Relaxed);
  }
}

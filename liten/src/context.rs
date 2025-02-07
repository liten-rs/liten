use std::sync::{
  atomic::{AtomicBool, AtomicUsize, Ordering},
  Arc, LazyLock, OnceLock,
};

use crate::{io_loop::IOEventLoop, task::Task};
use crossbeam::channel::Sender;

static CONTEXT: LazyLock<Context> = LazyLock::new(|| Context {
  has_entered: AtomicBool::new(false),
  sender: OnceLock::new(),
  current_task_id: AtomicUsize::new(0),
  current_reactor_id: AtomicUsize::new(0),
  io: IOEventLoop::init(),
});

pub struct Context {
  current_task_id: AtomicUsize,
  current_reactor_id: AtomicUsize,
  has_entered: AtomicBool,
  sender: OnceLock<Sender<Arc<Task>>>,
  io: IOEventLoop,
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send);

impl Context {
  pub fn io(&self) -> &IOEventLoop {
    &self.io
  }

  /// Returns the previous value
  pub fn task_id_inc(&self) -> usize {
    self.current_task_id.fetch_add(1, Ordering::SeqCst)
  }

  pub fn next_registration_token(&self) -> mio::Token {
    mio::Token(self.current_reactor_id.fetch_add(1, Ordering::SeqCst))
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

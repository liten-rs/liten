use std::{
  cell::{LazyCell, OnceCell},
  sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    Arc, LazyLock, OnceLock,
  },
};

use crate::runtime::scheduler;

use crate::{io_loop as io, task::Task};
use crossbeam::channel::Sender;

std::thread_local! {
  static HAS_CONTEXT_INIT: AtomicU8 = AtomicU8::new(0);

  static CONTEXT: LazyCell<Context> =LazyCell::new(|| {
    Context {
      has_entered: AtomicBool::new(false),
      handle: OnceLock::new(),
      current_task_id: AtomicUsize::new(0),
    }
  });
}

pub struct Context {
  current_task_id: AtomicUsize,
  has_entered: AtomicBool,
  handle: OnceLock<scheduler::Handle>,
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send);

pub fn has_init() -> bool {
  HAS_CONTEXT_INIT.with(|v| v.load(Ordering::Relaxed) == 2) // HAS_CONTEXT_INIT.fetch_add gets called before
                                                            // CONTEXT can init, so if called twice it means it
                                                            // has already been init once.
}

impl Context {
  pub fn handle(&self) -> &scheduler::Handle {
    &self.handle.get().expect("Accessed the io driver before initializing")
  }

  /// Returns the previous value
  pub fn task_id_inc(&self) -> usize {
    self.current_task_id.fetch_add(1, Ordering::SeqCst)
  }
}

pub fn with_context<F, R>(func: F) -> R
where
  F: FnOnce(&LazyCell<Context>) -> R,
{
  CONTEXT.with(func)
}

pub fn runtime_enter<F, R>(handle: &scheduler::Handle, f: F)
where
  F: FnOnce(&LazyCell<Context>),
{
  with_context(|ctx| {
    if ctx.has_entered.load(Ordering::Relaxed) {
      panic!("crate 'liten' user error: can't nest 'liten::runtime::Runtime'");
    }

    ctx.has_entered.store(true, Ordering::Relaxed);
    ctx.handle.set(handle).unwrap();
    ctx.has_entered.store(true, Ordering::Relaxed);

    f(ctx);

    ctx.has_entered.store(false, Ordering::Relaxed);
    let _ = ctx.handle.take();
  });
}

pub struct ContextDropper;

impl Drop for ContextDropper {
  fn drop(&mut self) {
    with_context(|ctx| {
      ctx.has_entered.store(false, Ordering::Relaxed);
    });
  }
}

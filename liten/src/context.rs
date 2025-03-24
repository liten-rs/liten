use std::{cell::LazyCell, sync::OnceLock};

use crate::loom::sync::Arc;

use crate::runtime::scheduler;

crate::loom::thread_local! {
  static CONTEXT: LazyCell<Context> = LazyCell::new(|| {
    Context {
      handle: OnceLock::new(),
    }
  });
}

pub struct Context {
  handle: OnceLock<Arc<scheduler::Handle>>,
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send);

impl Context {
  pub fn handle(&self) -> Arc<scheduler::Handle> {
    self.handle.get().expect("Accessed the handle before initializing").clone()
  }
}

pub fn with_context<F, R>(func: F) -> R
where
  F: FnOnce(&LazyCell<Context>) -> R,
{
  CONTEXT.with(func)
}

pub fn runtime_enter<F, R>(handle: Arc<scheduler::Handle>, f: F) -> R
where
  F: FnOnce(&LazyCell<Context>) -> R,
{
  with_context(|ctx| {
    if ctx.handle.get().is_some_and(|x| x.has_entered()) {
      panic!("nested runtimes is not supported");
    }

    if ctx.handle.set(handle).is_err() {
      panic!("whaat");
    };
    let return_type = f(ctx);

    ctx.handle.get().unwrap().exit();

    return_type
  })
}

pub struct ContextDropper;

impl Drop for ContextDropper {
  fn drop(&mut self) {
    with_context(|ctx| {
      ctx.handle.get().unwrap().exit();
    });
  }
}

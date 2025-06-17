use std::cell::RefCell;

use crate::runtime::scheduler;

// RefCell is used because thread_local which means only this module manages CONTEXT.
// None means runtime context is not initialised. Some(...) means context exists.
crate::loom::thread_local! {
  static CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
}

pub struct Context {
  handle: scheduler::Handle,
}

#[cfg(test)]
static_assertions::assert_impl_all!(Context: Send);

impl Context {
  fn new(scheduler_handle: scheduler::Handle) -> Self {
    Self { handle: scheduler_handle }
  }

  pub fn handle(&self) -> &scheduler::Handle {
    &self.handle
  }
}

pub fn runtime_enter<F, R>(handle: scheduler::Handle, f: F) -> R
where
  F: FnOnce(&Context) -> R,
{
  CONTEXT.with(|ctx| {
    let mut _ctx = ctx.borrow_mut();

    if _ctx.is_some() {
      panic!("Nested runtimes is not supported");
    }

    *_ctx = Some(Context::new(handle));
    drop(_ctx);

    let _ctx = ctx.borrow();
    let return_type = f(_ctx.as_ref().unwrap());

    drop(_ctx);

    let mut _ctx = ctx.borrow_mut();
    *_ctx = None;

    return_type
  })
}

pub fn with_context<F, R>(func: F) -> R
where
  F: FnOnce(&Context) -> R,
{
  CONTEXT.with(|value| {
    let _value = value.borrow();
    let Some(value) = _value.as_ref() else {
      panic!("with_context tried before runtime_enter");
    };

    func(value)
  })
}

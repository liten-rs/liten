use std::sync::OnceLock;

use crate::runtime::scheduler;

crate::loom::thread_local! {
  static CONTEXT: OnceLock<Context> = OnceLock::new();
}

pub struct Context {
  handle: scheduler::Handle,
}

// #[cfg(test)]
// static_assertions::assert_impl_all!(Context: Send);

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
    if let Err(_) = ctx.set(Context::new(handle)) {
      panic!("Nested runtimes is not supported");
    }
    let return_type = f(ctx.get().unwrap());

    // TODO: some deinitialisation

    return_type
  })
}

pub fn with_context<F, R>(func: F) -> R
where
  F: FnOnce(&Context) -> R,
{
  CONTEXT.with(|value| {
    let Some(value) = value.get() else {
      panic!("with_context tried before runtime_enter");
    };

    func(value)
  })
}
//
// pub fn runtime_enter<F, R>(handle: Arc<scheduler::Handle>, f: F) -> R
// where
//   F: FnOnce(&LazyCell<Context>) -> R,
// {
//   with_context(|ctx| {
//     if ctx.handle.get().is_some_and(|x| x.has_entered()) {
//       panic!("nested runtimes is not supported");
//     }
//
//     if ctx.handle.set(handle).is_err() {
//       panic!("whaat");
//     };
//     let return_type = f(ctx);
//
//     ctx.handle.get().unwrap().exit();
//
//     return_type
//   })
// }

// pub struct ContextDropper;
//
// impl Drop for ContextDropper {
//   fn drop(&mut self) {
//     with_context(|ctx| {
//       ctx.handle.get().expect("OnceLock handle get unwrapped").exit();
//     });
//   }
// }

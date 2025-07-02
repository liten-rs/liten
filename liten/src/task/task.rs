use std::{
  future::Future,
  panic::UnwindSafe,
  pin::Pin,
  task::{Context, Poll},
};

use crate::context::{self};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

impl Default for TaskId {
  fn default() -> Self {
    Self(context::with_context(|ctx| ctx.handle().task_id_inc()))
  }
}

impl TaskId {
  pub fn new() -> Self {
    Self::default()
  }
}

pub struct Task {
  id: TaskId,
  raw: *mut (),
  vtable: TaskVTable,
}

impl UnwindSafe for Task {}
// SAFETY: Task is only used in a single thread at any time.
unsafe impl Sync for Task {}
unsafe impl Send for Task {}

#[cfg(test)]
static_assertions::assert_impl_all!(Task: Send, Sync);

impl Task {
  pub(super) fn new<F>(id: TaskId, future: F) -> Task
  where
    F: Future<Output = ()> + Send + 'static,
  {
    Self {
      id,
      raw: Box::into_raw(Box::new(future)) as *mut (),
      vtable: TaskVTable { poll_fn: poll_fn::<F>, drop_fn: drop_fn::<F> },
    }
  }

  pub fn id(&self) -> TaskId {
    self.id
  }

  pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
    unsafe { (self.vtable.poll_fn)(self.raw, cx) }
  }
}

impl Drop for Task {
  fn drop(&mut self) {
    unsafe { (self.vtable.drop_fn)(self.raw) }
  }
}

struct TaskVTable {
  poll_fn: unsafe fn(*mut (), &mut Context<'_>) -> Poll<()>,
  drop_fn: unsafe fn(*mut ()),
}

unsafe fn poll_fn<F: Future<Output = ()>>(
  ptr: *mut (),
  cx: &mut Context<'_>,
) -> Poll<()> {
  Pin::new_unchecked(&mut *(ptr as *mut F)).poll(cx)
}

unsafe fn drop_fn<F: Future<Output = ()>>(ptr: *mut ()) {
  let _ = Box::from_raw(ptr as *mut F);
}

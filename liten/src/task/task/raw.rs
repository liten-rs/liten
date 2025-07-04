use std::{
  future::Future,
  panic::UnwindSafe,
  pin::Pin,
  task::{Context, Poll},
};

pub struct RawTask {
  raw: *mut (),
  vtable: TaskVTable,
}

impl UnwindSafe for RawTask {}
// SAFETY: Task is only used in a single thread at any time.
unsafe impl Sync for RawTask {}
unsafe impl Send for RawTask {}

#[cfg(test)]
static_assertions::assert_impl_all!(RawTask: Send, Sync);

impl RawTask {
  pub(super) fn from_future<F>(future: F) -> RawTask
  where
    F: Future<Output = ()> + 'static,
  {
    Self {
      // id: TaskId::new(),
      raw: Box::into_raw(Box::new(future)) as *mut (),
      vtable: TaskVTable { poll_fn: poll_fn::<F>, drop_fn: drop_fn::<F> },
    }
  }

  pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
    unsafe { (self.vtable.poll_fn)(self.raw, cx) }
  }
}

impl Drop for RawTask {
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
  // SAFETY: We own the owned value so not checking is safe.
  Pin::new_unchecked(&mut *(ptr as *mut F)).poll(cx)
}

unsafe fn drop_fn<F: Future<Output = ()>>(ptr: *mut ()) {
  let _ = Box::from_raw(ptr as *mut F);
}

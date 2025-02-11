use std::{
  future::Future,
  pin::Pin,
  sync::Mutex,
  task::{Context, Poll},
};

use crate::{
  context::{self},
  sync::oneshot::Sender,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

impl TaskId {
  pub fn new() -> Self {
    Self(context::with_context(|ctx| ctx.handle().task_id_inc()))
  }
}

pub struct Task {
  id: TaskId,
  pub future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

#[cfg(test)]
static_assertions::assert_impl_all!(Task: Send, Sync);

impl std::fmt::Debug for Task {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Task")
      .field("id", &self.id)
      .field("future", &"{...}")
      .finish()
  }
}

impl Task {
  pub(super) fn new<F>(id: TaskId, future: F, sender: Sender<F::Output>) -> Task
  where
    F: Future + Send + 'static,
    F::Output: Send,
  {
    let future = Box::pin(async move {
      let fut = future.await;
      if sender.send(fut).is_err() {
        // Ignore, task handler has been dropped in this case.
      }
    });
    Self { id, future: Mutex::new(future) }
  }

  pub fn id(&self) -> TaskId {
    self.id
  }

  pub fn poll(&self, cx: &mut Context) -> Poll<()> {
    let mut future_lock = self.future.lock().unwrap();
    future_lock.as_mut().poll(cx)
  }
}

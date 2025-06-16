use std::future::Future;

use crate::{context, sync::oneshot};

use super::{Task, TaskHandle, TaskId};

pub struct Builder {
  id: TaskId,
  name: Option<String>,
}

impl Default for Builder {
  fn default() -> Self {
    Builder { id: TaskId::new(), name: None }
  }
}

impl Builder {
  pub fn new() -> Self {
    Self::default()
  }
  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }
  pub fn build<F>(self, fut: F) -> TaskHandle<F::Output>
  where
    F: Future + Send + 'static,
    F::Output: Send,
  {
    let (write, read) = oneshot::channel();

    let task = Task::new(self.id, fut, write);
    context::with_context(|ctx| {
      ctx.handle().state().push_task(task);
    });

    TaskHandle(read)
  }
}

pub fn builder() -> Builder {
  Builder::new()
}

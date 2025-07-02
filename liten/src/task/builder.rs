use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use thiserror::Error;

use crate::{context, sync::oneshot};

use super::{Task, TaskId};

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
  pub fn spawn<F>(self, fut: F) -> TaskHandle<F::Output>
  where
    F: Future + Send + 'static,
    F::Output: Send,
  {
    let (sender, receiver) = oneshot::channel::<F::Output>();
    let task = Task::new(self.id, TaskFuture::new(fut, sender));

    context::with_context(|ctx| {
      ctx.handle().state().push_task(task);
    });

    TaskHandle(receiver)
  }
}

pin_project_lite::pin_project! {

  pub(crate) struct TaskFuture<F>
  where
    F: Future,
  {
    #[pin]
    fut: F,
    sender: Option<oneshot::Sender<F::Output>>,
  }
}

impl<F> TaskFuture<F>
where
  F: Future,
{
  fn new(fut: F, sender: oneshot::Sender<F::Output>) -> Self {
    Self { fut, sender: Some(sender) }
  }
}
impl<F> Future for TaskFuture<F>
where
  F: Future,
{
  type Output = ();
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.project();

    match this.fut.poll(cx) {
      Poll::Pending => Poll::Pending,
      Poll::Ready(value) => {
        // Ignore
        // receiver dropping
        let _ = this
          .sender
          .take()
          .expect("future polled after completion")
          .send(value);
        Poll::Ready(())
      }
    }
  }
}

pub struct TaskHandle<Out>(pub(super) oneshot::Receiver<Out>);

#[derive(Error, Debug, PartialEq)]
pub enum TaskHandleError {
  #[error("task panicked")]
  BodyPanicked,
}

impl<Out> Future for TaskHandle<Out>
where
  Out: 'static,
{
  type Output = Result<Out, TaskHandleError>;
  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let mut pinned = std::pin::pin!(&mut self.0);
    pinned.as_mut().poll(cx).map_err(|_| TaskHandleError::BodyPanicked)
  }
}

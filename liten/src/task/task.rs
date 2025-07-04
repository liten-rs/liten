mod raw;

use std::{
  future::Future,
  pin::Pin,
  sync::atomic::{AtomicUsize, Ordering},
  task::{Context, Poll},
};

pub use crate::sync::oneshot;
use thiserror::Error;
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

static CURRENT_TASK_ID: AtomicUsize = AtomicUsize::new(0);

impl Default for TaskId {
  fn default() -> Self {
    Self(CURRENT_TASK_ID.fetch_add(1, Ordering::SeqCst))
  }
}

impl TaskId {
  fn new() -> Self {
    Self::default()
  }
}

pub struct Task {
  id: TaskId,
  raw: raw::RawTask,
}

impl Task {
  pub fn new<Fut, Res>(fut: Fut) -> (Self, TaskHandle<Res>)
  where
    Fut: Future<Output = Res> + 'static,
  {
    let (task_future, handle) = TaskFuture::new(fut);
    let this =
      Task { id: TaskId::new(), raw: raw::RawTask::from_future(task_future) };

    (this, handle)
  }

  pub fn id(&self) -> TaskId {
    self.id
  }

  pub fn poll(&mut self, cx: &mut std::task::Context) -> Poll<()> {
    self.raw.poll(cx)
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
  fn new(fut: F) -> (Self, TaskHandle<F::Output>) {
    let (sender, receiver) = oneshot::channel::<F::Output>();
    let this = Self { fut, sender: Some(sender) };
    let handle = TaskHandle(receiver);

    (this, handle)
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

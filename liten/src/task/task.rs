mod raw;
mod state;

use std::{
  future::Future,
  pin::Pin,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
  task::{Context, Poll},
};

use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TaskId(pub usize);

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

  pub(crate) fn id(&self) -> TaskId {
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
    state: Arc<state::TaskResultState<F::Output>>,
  }

  impl<F: Future> PinnedDrop for TaskFuture<F> {
    fn drop(this: Pin<&mut Self>) {
      this.state.set_panicked();
    }
  }
}

impl<F> TaskFuture<F>
where
  F: Future,
{
  fn new(fut: F) -> (Self, TaskHandle<F::Output>) {
    // Single Arc allocation - this is the minimal allocation needed for soundness
    let state = Arc::new(state::TaskResultState::new());

    let this = Self { fut, state: state.clone() };
    let handle = TaskHandle::new(state);

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
        this.state.set_ready(value);
        Poll::Ready(())
      }
    }
  }
}

/// Task handle with sound lifetime management
pub struct TaskHandle<Out> {
  state: Arc<state::TaskResultState<Out>>,
}

impl<Out> TaskHandle<Out> {
  fn new(state: Arc<state::TaskResultState<Out>>) -> Self {
    Self { state }
  }
}

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
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    // Set the waker
    self.state.set_waker(cx.waker().clone());

    // Try to take the result
    match self.state.try_take() {
      Some(result) => Poll::Ready(result),
      None => Poll::Pending,
    }
  }
}

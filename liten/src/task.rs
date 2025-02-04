use std::{
  future::{Future, IntoFuture},
  pin::Pin,
  sync::{Arc, Mutex},
};

use futures_task::ArcWake;

use crate::context;

pub struct Task {
  pub future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}
static_assertions::assert_impl_all!(Task: Send);

impl ArcWake for Task {
  fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
    context::get_context_mut().push_task(arc_self.clone())
  }
}

impl Task {
  fn new<F>(future: F) -> Task
  where
    F: Future<Output = ()> + Send + 'static,
  {
    Self { future: Mutex::new(Box::pin(future)) }
  }
}

pub fn spawn<F>(fut: F) -> TaskHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send,
{
  let (write, read) = oneshot::channel::<F::Output>();

  let fut = Box::pin(async move {
    let task_return = fut.await;
    if write.send(task_return).is_err() {
      // Ignore, task handler has been dropped in this case.
    }
  });

  let task = Task::new(fut);
  context::get_context_mut().push_task(Arc::new(task));

  TaskHandle(read)
}

pub struct TaskHandle<Out>(oneshot::Receiver<Out>);

impl<Out> IntoFuture for TaskHandle<Out>
where
  Out: 'static,
{
  type Output = Out;
  type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;
  fn into_future(self) -> Self::IntoFuture {
    Box::pin(async move {
      match self.0.await {
        Ok(value) => value,
        Err(err) => unreachable!(), // I think?
      }
    })
  }
}

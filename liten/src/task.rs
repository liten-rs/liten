use std::{
  cell::RefCell,
  future::{Future, IntoFuture},
  pin::Pin,
  sync::{Arc, Mutex},
};

use tracing_futures::Instrument;

use oneshot::Sender;

use crate::context;

pub struct Task {
  id: usize,
  pub future: RefCell<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

unsafe impl Sync for Task {}

#[cfg(test)]
static_assertions::assert_impl_all!(Task: Send, Sync);

impl Task {
  fn new<F>(future: F, sender: Sender<F::Output>) -> Task
  where
    F: Future + Send + 'static,
    F::Output: Send,
  {
    let id = context::get_context().task_id_inc();
    let future = Box::pin(async move {
      let fut =
        future.instrument(tracing::trace_span!("liten task_id: ", %id)).await;
      if sender.send(fut).is_err() {
        // Ignore, task handler has been dropped in this case.
      }
    });
    Self { id, future: RefCell::new(future) }
  }

  pub fn id(&self) -> usize {
    self.id
  }
}

pub fn spawn<F>(fut: F) -> TaskHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send,
{
  let (write, read) = oneshot::channel::<F::Output>();
  let task = Task::new(fut, write);
  context::get_context().push_task(Arc::new(task));

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
        Err(_) => unreachable!(), // I think?
      }
    })
  }
}

use std::{
  future::{Future, IntoFuture},
  pin::Pin,
  sync::{Arc, Mutex},
};

use oneshot::Sender;

use crate::context;

pub struct Task {
  id: usize,
  pub future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}
#[cfg(test)]
static_assertions::assert_impl_all!(Task: Send);

impl Task {
  fn new<F>(future: F, sender: Sender<F::Output>) -> Task
  where
    F: Future + Send + 'static,
    F::Output: Send,
  {
    let context = context::get_context();
    let id = context.task_id_inc();

    let future = Box::pin(async move {
      if sender.send(future.await).is_err() {
        // Ignore, task handler has been dropped in this case.
      }
    });
    Self { id, future: Mutex::new(Box::pin(future)) }
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

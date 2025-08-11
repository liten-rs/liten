use std::{
  future::Future,
  task::{Context, Poll},
};

use crate::{
  future::block_on::park_waker,
  loom::thread,
  runtime::{scheduler::Scheduler, waker::create_task_waker},
  task::TaskStore,
};

pub struct SingleThreaded;

impl Scheduler for SingleThreaded {
  fn block_on<F>(&self, fut: F) -> F::Output
  where
    F: Future,
  {
    let mut fut = std::pin::pin!(fut);
    let _thread = thread::current();

    let waker = park_waker(_thread.clone());
    if let Poll::Ready(value) =
      fut.as_mut().poll(&mut Context::from_waker(&waker))
    {
      return value;
    }

    loop {
      TaskStore::get().move_cold_to_hot();
      while let Some(task) = TaskStore::get().task_dequeue() {
        let waker = create_task_waker(_thread.clone(), task.id());
        task.poll(&mut Context::from_waker(&waker));
      }

      let waker = park_waker(_thread.clone());
      if let Poll::Ready(value) =
        fut.as_mut().poll(&mut Context::from_waker(&waker))
      {
        return value;
      }

      lio::tick();

      thread::park();
    }
  }
}

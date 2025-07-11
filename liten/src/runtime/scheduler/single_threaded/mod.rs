use std::{
  future::Future,
  task::{Context, Poll},
};

use crate::{
  future::block_on::park_waker,
  runtime::{scheduler::Scheduler, waker::create_task_waker},
  task::TaskStore,
};

pub struct SingleThreaded;

impl Scheduler for SingleThreaded {
  fn schedule(task: crate::task::Task) {
    TaskStore::get().task_enqueue(task);
  }
  fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    let mut fut = std::pin::pin!(fut);

    let parker = parking::Parker::new();
    let unparker = parker.unparker();

    let waker = park_waker(unparker.clone());

    if let Poll::Ready(value) =
      fut.as_mut().poll(&mut Context::from_waker(&waker))
    {
      return value;
    }

    loop {
      TaskStore::get().move_cold_to_hot();
      while let Some(task) = TaskStore::get().task_dequeue() {
        let waker = create_task_waker(unparker.clone(), task.id());
        task.poll(&mut Context::from_waker(&waker));
      }

      let waker = park_waker(parker.unparker());
      if let Poll::Ready(value) =
        fut.as_mut().poll(&mut Context::from_waker(&waker))
      {
        return value;
      }

      parker.park();
    }
  }
}

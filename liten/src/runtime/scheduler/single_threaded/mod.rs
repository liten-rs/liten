use std::{
  future::{Future, IntoFuture},
  task::{Context, Poll},
};

use crate::{
  future::block_on::park_waker,
  runtime::{scheduler::Scheduler, waker::create_task_waker},
  task::TaskStore,
};

pub struct SingleThreaded;

impl Scheduler for SingleThreaded {
  fn block_on<F>(self, fut: F) -> F::Output
  where
    F: Future,
  {
    let mut fut = std::pin::pin!(fut.into_future());

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
      loop {
        match TaskStore::get().task_dequeue() {
          Some(task) => {
            let waker = create_task_waker(unparker.clone(), task.id());
            task.poll(&mut Context::from_waker(&waker));
          }
          None => break,
        };
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

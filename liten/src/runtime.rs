use std::{
  future::Future,
  pin::pin,
  sync::Arc,
  task::{Context as StdContext, Poll},
};

use crossbeam::channel::{self, TryRecvError};

use futures_task::{waker_ref, ArcWake};

use crate::context;

pub struct GlobalWaker(channel::Sender<()>);

impl ArcWake for GlobalWaker {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self.0.send(()).unwrap();
  }
}

pub struct Runtime;

impl Runtime {
  pub fn new() -> Self {
    Runtime
  }

  pub fn block_on<F, Res>(&mut self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    let _entered = context::enter();

    let (sender, receiver) = channel::unbounded();

    let binding = Arc::new(GlobalWaker(sender));
    let waker = waker_ref(&binding);
    let mut context = StdContext::from_waker(&waker);
    let mut pinned = pin!(fut);

    loop {
      if let Poll::Ready(value) = pinned.as_mut().poll(&mut context) {
        return value;
      }

      let mut ctx = context::get_context_mut();
      if let Some(task) = ctx.pop_task() {
        let waker = waker_ref(&task);
        let mut context = StdContext::from_waker(&waker);

        let task_to_poll = task.clone();

        let mut task_lock = task_to_poll.future.lock().unwrap();
        if let Poll::Pending = task_lock.as_mut().poll(&mut context) {
          ctx.push_task(task);
        }
      }

      if let Err(err) = receiver.try_recv() {
        match err {
          TryRecvError::Empty => {}
          TryRecvError::Disconnected => unreachable!(),
        }
      }
    }
  }
}

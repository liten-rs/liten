use std::{
  cell::{Cell, UnsafeCell},
  future::Future,
  panic::UnwindSafe,
  pin::{self as stdpin, Pin},
  task::{Context, Poll},
};

use crate::{
  context::{self},
  sync::oneshot::Sender,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

impl Default for TaskId {
  fn default() -> Self {
    Self(context::with_context(|ctx| ctx.handle().task_id_inc()))
  }
}

impl TaskId {
  pub fn new() -> Self {
    Self::default()
  }
}

pub struct Task {
  id: TaskId,
  pub future: Cell<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
}

impl UnwindSafe for Task {}
// SAFETY: Task is only used in a single thread at any time.
unsafe impl Sync for Task {}

#[cfg(test)]
static_assertions::assert_impl_all!(Task: Send, Sync);

impl Task {
  pub(super) fn new<F>(id: TaskId, future: F, sender: Sender<F::Output>) -> Task
  where
    F: Future + Send + 'static,
    F::Output: Send,
  {
    let future = Box::pin(async move {
      let fut = future.await;
      if sender.send(fut).is_err() {
        // Ignore, task handler has been dropped in this case.
      }
    });
    Self { id, future: Cell::new(Some(future)) }
  }

  pub fn id(&self) -> TaskId {
    self.id
  }

  pub fn poll(&self, cx: &mut Context) -> Poll<()> {
    let mut future = self
      .future
      .take()
      .expect(&format!("Future::poll called on Task id {:?}", self.id()));

    let poll = stdpin::pin!(&mut future).poll(cx);

    if poll == Poll::Pending {
      self.future.set(Some(future));
    }

    poll
  }
}

// #[cfg(test)]
// mod tests {
//   use super::*;
//   use std::cell::Cell;
//   use std::future::{ready, Future};
//   use std::pin::Pin;
//   use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
//
//   fn dummy_waker() -> Waker {
//     fn no_op(_: *const ()) {}
//     fn clone(_: *const ()) -> RawWaker {
//       dummy_raw_waker()
//     }
//     static VTABLE: RawWakerVTable =
//       RawWakerVTable::new(clone, no_op, no_op, no_op);
//     fn dummy_raw_waker() -> RawWaker {
//       RawWaker::new(std::ptr::null(), &VTABLE)
//     }
//     unsafe { Waker::from_raw(dummy_raw_waker()) }
//   }

// Fix handler first
// #[crate::internal_test]
// fn task_poll_ready() {
//   let (sender, receiver) = crate::sync::oneshot::channel();
//   let task = Task::new(TaskId::new(), async { 42 }, sender);
//   let waker = dummy_waker();
//   let mut cx = Context::from_waker(&waker);
//   let poll = task.poll(&mut cx);
//   assert!(matches!(poll, Poll::Ready(())));
//   assert_eq!(receiver.try_recv().unwrap(), Some(42));
// }
// }

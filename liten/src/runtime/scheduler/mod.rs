pub mod worker;

use std::{
  cell::LazyCell,
  collections::HashMap,
  future::Future,
  sync::Arc,
  task::{Context as StdContext, Poll},
  thread,
  time::Duration,
};

use super::{super::io_loop as io, waker::RuntimeWaker};

use crate::{
  context::{self, Context},
  task::ArcTask,
};

#[derive(Debug)]
pub struct Scheduler;

pub struct Handle {
  pub io: io::Handle,
  pub shared: Arc<worker::Shared>,
}

impl Handle {
  pub fn new(io: io::Handle, state: Arc<worker::Shared>) -> Handle {
    Handle { io, shared: state }
  }

  pub fn push_task(&self, task: ArcTask) {
    self.shared.injector.push(task);
  }
}

pub struct Driver {
  pub io: io::Driver,
}

impl Handle {
  pub fn state(&self) -> &worker::Shared {
    &self.shared
  }
}

impl Handle {
  pub fn io(&self) -> &io::Handle {
    &self.io
  }
}

impl Scheduler {
  pub fn block_on<F, R>(&self, f: F) -> R
  where
    F: Future<Output = R>,
  {
    let runtime_waker = Arc::new(RuntimeWaker::new(thread::current())).into();
    let mut context = StdContext::from_waker(&runtime_waker);
    let mut pinned = std::pin::pin!(f);

    loop {
      match pinned.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => {
          println!("main sleepy");
          thread::park()
        }
      };
    }

    //todo!()
  }
  //pub fn from_receiver(receiver: Receiver<Arc<Task>>) -> Self {
  //  let (cth_sender, cth_reader) = crossbeam::channel::unbounded();
  //  Self {
  //    task_queue: Arc::new(Injector::new()),
  //    cold_task_queue: HashMap::new(),
  //    push_to_hot_receiver: receiver,
  //
  //    cold_to_hot_sender: cth_sender,
  //    cold_to_hot_receiver: cth_reader,
  //  }
  //}
  //pub fn tick(&mut self) {
  //  for task in self.push_to_hot_receiver.try_iter() {
  //    self.task_queue.push(task);
  //  }
  //  loop {
  //    match self.task_queue.steal() {
  //      Steal::Empty => break,
  //      Steal::Retry => continue,
  //      Steal::Success(task) => {
  //        let span = tracing::trace_span!("TaskId", id = task.id().0);
  //        let _span = span.enter();
  //        let waker = Arc::new(LitenWaker::new(
  //          task.id(),
  //          self.cold_to_hot_sender.clone(),
  //        ))
  //        .into();
  //        let mut context = StdContext::from_waker(&waker);
  //
  //        let task_to_send = task.clone();
  //        let mut task_lock = task.future.borrow_mut();
  //        if task_lock.as_mut().poll(&mut context).is_pending() {
  //          tracing::trace!("not making progress, moving to cold",);
  //
  //          self.cold_task_queue.insert(task_to_send.id(), task_to_send);
  //        };
  //        continue;
  //      }
  //    }
  //  }
  //  for task_id in self.cold_to_hot_receiver.try_iter() {
  //    let task_to_move = self
  //      .cold_task_queue
  //      .remove(&task_id)
  //      .expect("They should always be valid");
  //
  //    tracing::trace!("TaskId={:?} moved to hot queue", task_id);
  //    self.task_queue.push(task_to_move);
  //  }
  //}
}

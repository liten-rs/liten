pub mod worker;

use std::{
  collections::HashMap,
  sync::Arc,
  task::{Context as StdContext, Poll},
};

use super::super::io_loop as io;

use crate::{
  context,
  task::{ArcTask, Task, TaskId},
  taskqueue::TaskQueue,
};

use crossbeam::{
  channel::{Receiver, Sender},
  deque::{Injector, Steal},
};

use super::waker::LitenWaker;

#[derive(Debug)]
pub struct Scheduler;
/// The main "hot" queue. Tasks that are expected to make progress are here.
//task_queue: Arc<Injector<ArcTask>>,
//cold_task_queue: HashMap<TaskId, ArcTask>,
//push_to_hot_receiver: Receiver<ArcTask>,
//
//cold_to_hot_receiver: Receiver<TaskId>,
//cold_to_hot_sender: Sender<TaskId>,

pub struct Handle {
  pub io: io::Handle,
  pub shared: worker::Shared,
}

impl Handle {
  pub fn state(&self) -> worker::Shared {
    &self.shared
  }
}

impl Handle {
  pub fn io(&self) -> &io::Handle {
    &self.io
  }
}

impl Scheduler {
  pub fn block_on(&self, handle: &Handle) {
    context::runtime_enter(handle)
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

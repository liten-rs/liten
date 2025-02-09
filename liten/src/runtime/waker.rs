use std::{
  sync::Arc,
  task::{RawWaker, RawWakerVTable, Wake},
};

use crate::task::{Task, TaskId};

pub struct LitenWaker {
  task_id: TaskId,
  sender: crossbeam::channel::Sender<TaskId>,
}

impl LitenWaker {
  pub(crate) fn new(
    task: TaskId,
    sender: crossbeam::channel::Sender<TaskId>,
  ) -> Self {
    Self { task_id: task, sender }
  }
}

impl Wake for LitenWaker {
  fn wake(self: Arc<Self>) {
    self.sender.send(self.task_id).unwrap();
  }
}

// Waker implementation to notify the runtime
pub struct RuntimeWaker {
  sender: crossbeam::channel::Sender<()>,
}

impl RuntimeWaker {
  pub fn new(sender: crossbeam::channel::Sender<()>) -> Self {
    Self { sender }
  }
}

impl Wake for RuntimeWaker {
  fn wake(self: Arc<Self>) {
    self.sender.send(()).unwrap();
  }
}

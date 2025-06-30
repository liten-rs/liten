use std::task::Wake;

use parking::Unparker;

use crate::loom::thread::Thread;

use crate::task::TaskId;
use std::sync::mpsc;

pub struct TaskWaker {
  task_id: TaskId,
  sender: mpsc::Sender<TaskId>,
  unparker: Unparker,
}

impl TaskWaker {
  pub(crate) fn new(
    task: TaskId,
    sender: mpsc::Sender<TaskId>,
    unparker: Unparker,
  ) -> Self {
    Self { task_id: task, sender, unparker }
  }
}

impl Wake for TaskWaker {
  fn wake(self: std::sync::Arc<Self>) {
    self.sender.send(self.task_id).unwrap();
    self.unparker.unpark();
  }
}

// Waker implementation to notify the runtime
pub struct RuntimeWaker(Thread);

impl RuntimeWaker {
  pub fn new(thread: Thread) -> Self {
    Self(thread)
  }
}

impl Wake for RuntimeWaker {
  fn wake(self: std::sync::Arc<Self>) {
    self.0.unpark();
  }
}

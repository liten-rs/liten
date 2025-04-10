// loom::Arc can't be used here because of Into<Waker> for std::Arc.

use std::task::Wake;

use crate::loom::thread::Thread;

use crate::{sync::mpsc, task::TaskId};

pub struct TaskWaker {
  task_id: TaskId,
  sender: mpsc::Sender<TaskId>,
}

impl TaskWaker {
  pub(crate) fn new(task: TaskId, sender: mpsc::Sender<TaskId>) -> Self {
    Self { task_id: task, sender }
  }
}

impl Wake for TaskWaker {
  fn wake(self: std::sync::Arc<Self>) {
    tracing::trace!(task_id = ?self.task_id, "task wake called");
    self.sender.send(self.task_id).unwrap();
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

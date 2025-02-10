use std::{sync::Arc, task::Wake, thread::Thread};

use crate::task::TaskId;

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
pub struct RuntimeWaker(Thread);

impl RuntimeWaker {
  pub fn new(thread: Thread) -> Self {
    Self(thread)
  }
}

impl Wake for RuntimeWaker {
  fn wake(self: Arc<Self>) {
    println!("main unsleepy");
    self.0.unpark();
  }
}

use async_task::Runnable;
use crossbeam_queue::ArrayQueue;

pub(crate) struct TaskStore {
  task_queue: ArrayQueue<Runnable>,
}

impl TaskStore {
  pub(crate) fn new() -> Self {
    TaskStore { task_queue: ArrayQueue::new(2048) }
  }

  pub fn task_enqueue(&self, task: Runnable) {
    let result = self.task_queue.push(task);
    // For now
    assert!(result.is_ok(), "exceeded the 2048 limit");
  }

  pub fn pop(&self) -> Option<Runnable> {
    self.task_queue.pop()
  }
}

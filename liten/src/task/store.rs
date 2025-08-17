use std::sync::OnceLock;

use async_task::Runnable;
use crossbeam_queue::ArrayQueue;

pub struct TaskStore {
  task_queue: ArrayQueue<Runnable>,
}

impl TaskStore {
  pub fn get() -> &'static Self {
    static TASK_STORE: OnceLock<TaskStore> = OnceLock::new();
    TASK_STORE.get_or_init(Self::new)
  }

  pub(crate) fn new() -> Self {
    TaskStore { task_queue: ArrayQueue::new(2048) }
  }

  pub fn task_enqueue(&self, task: Runnable) {
    let result = self.task_queue.push(task);
    // For now
    assert!(result.is_ok(), "exceeded the 2048 limit");
  }

  pub fn tasks(&self) -> impl Iterator<Item = Runnable> {
    let mut tasks = Vec::new();
    while let Some(task) = self.task_queue.pop() {
      tasks.push(task);
    }

    tasks.into_iter()
  }
}

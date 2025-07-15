use std::{
  collections::HashMap,
  mem,
  sync::{Mutex, OnceLock},
};

use crossbeam_queue::ArrayQueue;

use crate::task::{Task, TaskId};

pub struct TaskStore {
  field1: Mutex<TaskStoreInner>,
  task_queue: ArrayQueue<Task>,
}

struct TaskStoreInner {
  cold: HashMap<TaskId, Task>,
  cold_to_hot: Vec<TaskId>,
}

impl TaskStore {
  pub fn get() -> &'static Self {
    static TASK_STORE: OnceLock<TaskStore> = OnceLock::new();

    // Not using get here since TaskStore isn't something that should be a choosen api.
    TASK_STORE.get_or_init(|| TaskStore {
      task_queue: ArrayQueue::new(512),
      field1: Mutex::new(TaskStoreInner {
        cold: HashMap::new(),
        cold_to_hot: Vec::new(),
      }),
    })
  }

  pub fn task_enqueue(&self, task: Task) {
    // For now
    assert!(!self.task_queue.push(task).is_err(), "exceeded the 512 limit");
  }

  pub fn task_dequeue(&self) -> Option<Task> {
    self.task_queue.pop()
  }

  pub(crate) fn insert_cold(&self, task: Task) {
    self.field1.lock().unwrap().cold.insert(task.id(), task);
  }

  pub fn wake_task(&self, task_id: TaskId) {
    let mut lock = self.field1.lock().unwrap();

    lock.cold_to_hot.push(task_id);
  }

  pub fn move_cold_to_hot(&self) {
    let mut lock = self.field1.lock().unwrap();

    let testing = mem::take(&mut lock.cold_to_hot);

    for task_id in testing {
      if let Some(task) = lock.cold.remove(&task_id) {
        self.task_enqueue(task);
      }
    }
  }
}

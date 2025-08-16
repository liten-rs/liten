use std::{
  collections::HashMap,
  mem,
  sync::{Mutex, OnceLock},
};

use async_task::Runnable;
use crossbeam_queue::ArrayQueue;

pub struct TaskStore {
  // field1: Mutex<TaskStoreInner>,
  task_queue: ArrayQueue<Runnable>,
}

struct TaskStoreInner {
  // cold: HashMap<TaskId, Task>,
  // cold_to_hot: Vec<TaskId>,
}

impl TaskStore {
  pub fn get() -> &'static Self {
    static TASK_STORE: OnceLock<TaskStore> = OnceLock::new();

    // Not using get here since TaskStore isn't something that should be a choosen api.
    TASK_STORE.get_or_init(|| TaskStore {
      task_queue: ArrayQueue::new(2048),
      // field1: Mutex::new(TaskStoreInner {
      //   cold: HashMap::new(),
      //   cold_to_hot: Vec::new(),
      // }),
    })
  }

  pub fn task_enqueue(&self, task: Runnable) {
    // For now
    let result = self.task_queue.push(task);
    assert!(result.is_ok(), "exceeded the 2048 limit");
  }

  pub fn tasks(&self) -> impl Iterator<Item = Runnable> {
    let mut tasks = Vec::new();
    while let Some(task) = self.task_queue.pop() {
      tasks.push(task);
    }

    tasks.into_iter()
    // self.task_queue.into_iter()
  }

  // pub(crate) fn insert_cold(&self, task: Runnable) {
  // self.
  // self.field1.lock().unwrap().cold.insert(task.id(), task);
  // }

  // pub fn wake_task(&self, task_id: TaskId) {
  //   let mut lock = self.field1.lock().unwrap();
  //
  //   lock.cold_to_hot.push(task_id);
  // }

  // pub fn move_cold_to_hot(&self) {
  //   let mut lock = self.field1.lock().unwrap();
  //
  //   let testing = mem::take(&mut lock.cold_to_hot);
  //
  //   for task_id in testing {
  //     if let Some(task) = lock.cold.remove(&task_id) {
  //       self.task_enqueue(task);
  //     }
  //   }
  // }
}

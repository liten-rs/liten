use std::{collections::HashMap, task::Poll};

use crossbeam_deque::{Steal, Worker as WorkerQueue};
use crossbeam_utils::sync::Parker;

use crate::{
  loom::sync::Arc,
  runtime::{scheduler::Handle, waker::TaskWaker},
  sync::{
    mpsc,
    oneshot::{self, Receiver},
  },
  task::{Task, TaskId},
};

// Local worker.
pub struct Worker {
  worker_id: usize,
  handle: Handle,
  parker: Parker,

  local_queue: WorkerQueue<Task>,
  cold_queue: HashMap<TaskId, Task>,

  receiver: Receiver<()>,
}

impl Worker {
  pub fn new(id: usize, handle: Handle) -> Worker {
    let (sender, receiver) = oneshot::channel();
    drop(sender);
    Worker {
      worker_id: id,
      handle,
      parker: Parker::new(),
      receiver,
      cold_queue: HashMap::new(),
      local_queue: WorkerQueue::new_fifo(),
    }
  }

  pub fn id(&self) -> usize {
    self.worker_id
  }

  pub fn parker(&self) -> &Parker {
    &self.parker
  }

  pub fn stealer(&self) -> crossbeam_deque::Stealer<Task> {
    self.local_queue.stealer()
  }

  pub fn get_shutdown_sender(&self) -> oneshot::Sender<()> {
    self.receiver.try_get_sender().unwrap()
  }

  fn fetch_task(&self) -> Option<Task> {
    if let Some(task) = self.local_queue.pop() {
      tracing::trace!(task_id = ?task.id(), "fetched local task");
      return Some(task);
      // Fill local queue from the global tasks
    };

    // Try to steal tasks from the global queue
    loop {
      match self.steal_from_global_queue() {
        Steal::Retry => continue,
        Steal::Success(task) => return Some(task),
        Steal::Empty => break,
      };
    }

    // Global queue is empty: So we steal tasks from other workers.

    let iter = self.handle.state().remotes.iter();
    for remote_worker in iter {
      loop {
        // Steal workers and pop the local queue
        match remote_worker.stealer.steal_batch_and_pop(&self.local_queue) {
          // Try again with same remote
          Steal::Retry => continue,
          // Stop trying and move on to the next one.
          Steal::Empty => break,
          // Break immediately and return task
          Steal::Success(task) => {
            tracing::trace!("hehe stole task");
            return Some(task);
          }
        }
      }
    }

    None
  }

  fn steal_from_global_queue(&self) -> Steal<Task> {
    self.handle.state().injector.steal_batch_and_pop(&self.local_queue)
  }
  pub fn launch(&mut self) {
    let (sender, receiver) = mpsc::unbounded();
    tracing::trace!(worker_id = self.id(), "starting");
    loop {
      if self.receiver.try_recv().is_ok() {
        tracing::trace!(worker_id = self.id(), "shutting down");
        break;
      }
      for now_active_task_id in receiver.try_iter() {
        tracing::trace!(task_id = ?now_active_task_id, "moving task from cold_queue to local_queue");
        let task = self
          .cold_queue
          .remove(&now_active_task_id)
          .expect("invalid waker called, TaskId doesn't exist");

        self.local_queue.push(task);
      }

      let Some(task) = self.fetch_task() else {
        self.parker.park();
        continue;
      };
      let id = task.id();
      let liten_waker =
        std::sync::Arc::new(TaskWaker::new(id, sender.clone())).into();
      let mut context = std::task::Context::from_waker(&liten_waker);

      let poll_result =
        std::panic::catch_unwind(move || match task.poll(&mut context) {
          Poll::Pending => UnwindTaskResult::Pending(task),
          Poll::Ready(()) => UnwindTaskResult::Ok,
        });

      if let Ok(UnwindTaskResult::Pending(task)) = poll_result {
        tracing::trace!(task_id = ?task.id(), "moving to cold_queue");
        let old_value = self.cold_queue.insert(id, task);
        assert!(old_value.is_none(), "logic error of inserted cold_queue task");
      }
    }
  }
}

enum UnwindTaskResult {
  Pending(Task),
  Ok,
}

use std::{collections::HashMap, sync::Arc, task::Poll};

use crossbeam_deque::{Steal, Worker as WorkerQueue};
use crossbeam_utils::sync::Parker;

use crate::{
  runtime::{scheduler::Handle, waker::TaskWaker},
  sync::{mpsc, oneshot::Receiver},
  task::{ArcTask, TaskId},
};

pub struct WorkerBuilder {
  worker_id: usize,
  handle: Option<Arc<Handle>>,
  parker: Option<Parker>,

  queue: Option<WorkerQueue<ArcTask>>,
}

impl WorkerBuilder {
  pub fn with_id(worker_id: usize) -> Self {
    WorkerBuilder { worker_id, handle: None, parker: None, queue: None }
  }

  pub fn handle(mut self, handle: Arc<Handle>) -> Self {
    self.handle = Some(handle);
    self
  }

  pub fn parker(mut self, parker: Parker) -> Self {
    self.parker = Some(parker);
    self
  }

  pub fn queue(mut self, queue: WorkerQueue<ArcTask>) -> Self {
    self.queue = Some(queue);
    self
  }

  pub fn build(self, receiver: Receiver<()>) -> Worker {
    Worker {
      worker_id: self.worker_id,
      handle: self.handle.expect("handle is required"),
      parker: self.parker.expect("parker is required"),

      local_queue: self.queue.expect("queue is required"),
      cold_queue: HashMap::new(),
      receiver,
    }
  }
}

// Local worker.
pub struct Worker {
  worker_id: usize,
  handle: Arc<Handle>,
  parker: crossbeam_utils::sync::Parker,

  local_queue: WorkerQueue<ArcTask>,
  cold_queue: HashMap<TaskId, ArcTask>,

  receiver: Receiver<()>,
}

impl Worker {
  pub fn id(&self) -> usize {
    self.worker_id
  }
  fn fetch_task(&self) -> Option<ArcTask> {
    if let Some(task) = self.local_queue.pop() {
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

  fn steal_from_global_queue(&self) -> Steal<ArcTask> {
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
      let liten_waker = Arc::new(TaskWaker::new(id, sender.clone())).into();
      let mut context = std::task::Context::from_waker(&liten_waker);

      let unwind_task = task.clone();
      let poll_result = match std::panic::catch_unwind(move || {
        unwind_task.poll(&mut context)
      }) {
        Ok(value) => value,
        Err(_) => continue,
      };

      if Poll::Pending == poll_result {
        let old_value = self.cold_queue.insert(id, task);
        assert!(old_value.is_none(), "logic error of inserted cold_queue task");
      }
    }
  }
}

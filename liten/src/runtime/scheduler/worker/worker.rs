use std::{collections::HashMap, task::Poll};

use crossbeam_deque::{Injector, Steal, Stealer, Worker as CBWorkerQueue};
use crossbeam_utils::sync::Parker;

use crate::{
  context::Context,
  runtime::waker::TaskWaker,
  sync::{
    mpsc,
    oneshot::{self, not_sync::OneshotError, Receiver},
  },
  task::{Task, TaskId},
};

#[derive(Debug)]
pub struct WorkerQueue(CBWorkerQueue<Task>);

impl WorkerQueue {
  pub fn new() -> Self {
    Self(CBWorkerQueue::new_fifo())
  }
  pub fn push(&self, task: Task) {
    self.0.push(task);
  }
  pub fn fetch_task(
    &self,
    injector: &Injector<Task>,
    remotes: &[Stealer<Task>],
  ) -> Option<Task> {
    if let Some(task) = self.0.pop() {
      return Some(task);
      // Fill local queue from the global tasks
    };

    // Try to steal tasks from the global queue
    loop {
      match injector.steal_batch_and_pop(&self.0) {
        Steal::Retry => continue,
        Steal::Success(task) => return Some(task),
        Steal::Empty => break,
      };
    }

    // Global queue is empty: So we steal tasks from other workers.
    for remote_stealer in remotes.into_iter() {
      loop {
        // Steal workers and pop the local queue
        match remote_stealer.steal_batch_and_pop(&self.0) {
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
}

// Local worker.
pub struct Worker {
  worker_id: usize,
  // handle: Handle,
  parker: Parker,

  hot: WorkerQueue,
  cold: HashMap<TaskId, Task>,

  pub shutdown_receiver: Receiver<()>,
}

impl Worker {
  pub fn new(id: usize) -> Worker {
    let (sender, receiver) = oneshot::channel();
    drop(sender);
    Worker {
      worker_id: id,
      parker: Parker::new(),
      hot: WorkerQueue::new(),
      cold: HashMap::new(),
      shutdown_receiver: receiver,
    }
  }

  pub fn id(&self) -> usize {
    self.worker_id
  }

  pub fn parker(&self) -> &Parker {
    &self.parker
  }

  pub fn get_shutdown_sender(&self) -> oneshot::Sender<()> {
    self.shutdown_receiver.try_get_sender().unwrap()
  }

  pub fn stealer(&self) -> Stealer<Task> {
    self.hot.0.stealer()
  }

  pub fn launch(&mut self, context: &Context) {
    let (sender, receiver) = mpsc::unbounded();
    let _guard = tracing::trace_span!("worker launch", worker_id = self.id());
    let _ye = _guard.enter();
    tracing::trace!(worker_id = self.id(), "starting");
    loop {
      match self.shutdown_receiver.try_recv() {
        Ok(Some(_)) => {
          tracing::trace!(worker_id = self.id(), "shutting down");
          break;
        }
        Ok(None) => {}
        Err(err) => match err {
          OneshotError::SenderDropped => {
            panic!("shutdown sender dropped before sending shutdown signal")
          }
          _ => unreachable!(),
        },
      };

      for now_active_task_id in receiver.try_iter() {
        tracing::trace!(task_id = ?now_active_task_id, "moving task from cold_queue to local_queue");
        let task = self
          .cold
          .remove(&now_active_task_id)
          .expect("invalid waker called, TaskId doesn't exist");

        self.hot.push(task);
      }

      let shared = context.handle().state();

      let stealers: Vec<Stealer<Task>> =
        shared.remotes().iter().map(|remote| remote.stealer.clone()).collect();

      let Some(task) = self.hot.fetch_task(shared.injector(), &stealers) else {
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
        let old_value = self.cold.insert(id, task);
        assert!(old_value.is_none(), "logic error of inserted cold_queue task");
      }
    }
  }
}

enum UnwindTaskResult {
  Pending(Task),
  Ok,
}

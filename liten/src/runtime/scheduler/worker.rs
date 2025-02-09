use std::sync::Arc;

use crossbeam::deque::{Injector, Stealer, Worker as WorkerQueue};

use crate::task::ArcTask;

use super::Handle;

pub struct Shared {
  pub remotes: Box<[Remote]>,
  pub inject: Injector<ArcTask>,
}

impl Shared {
  pub fn push_task(&self, task: ArcTask) {
    self.inject.push(task);
  }
}

// Local worker.
pub struct Worker {
  handle: Handle,
  shared: Arc<Shared>,
  local_queue: WorkerQueue<ArcTask>,
}

// One remote worker.
pub struct Remote {
  stealer: Stealer<ArcTask>,
}

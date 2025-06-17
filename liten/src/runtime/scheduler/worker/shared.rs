use std::sync::OnceLock;

use super::{worker::Worker, Remote};
use crossbeam_deque::Injector;

use crate::task::Task;

use crate::loom::sync::Arc;

#[derive(Debug, Clone)]
pub struct Shared(Arc<SharedInner>);

#[derive(Debug)]
struct SharedInner {
  // SAFETY: Only mutating before workers can access this. Mutating is only done init-time.
  remotes: Arc<OnceLock<Box<[Remote]>>>,
  injector: Arc<Injector<Task>>,
}

unsafe impl Sync for SharedInner {}

impl Shared {
  /// Make sure to call [`Shared::fill_remotes`] before using Shared.
  pub fn new_without_remotes() -> Self {
    Self(Arc::new(SharedInner {
      remotes: Arc::new(OnceLock::new()),
      injector: Arc::new(Injector::new()),
    }))
  }

  pub fn fill_remotes(&self, workers: &[Worker]) {
    let remotes = workers
      .iter()
      .map(|worker| {
        let stealer = worker.stealer();
        let unparker = worker.parker().unparker().clone();
        Remote::from_stealer(stealer, unparker)
      })
      .collect::<Vec<_>>()
      .into_boxed_slice();

    self
      .0
      .remotes
      .set(remotes)
      .expect("Coun't fill remotes. Maybe error: Only allowed to call once.");
  }

  pub fn push_task(&self, task: Task) {
    tracing::trace!("Pushed task");
    self.0.injector.push(task);

    for remote in self.0.remotes.get().unwrap().iter() {
      remote.unpark();
    }
  }

  pub fn remotes(&self) -> &[Remote] {
    self.0.remotes.get().expect("Remotes accessed before 'fill_remotes'")
  }

  pub fn injector(&self) -> &Injector<Task> {
    &self.0.injector
  }
}

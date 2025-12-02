use std::sync::OnceLock;

use super::{worker::Worker, Remote};

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Shared(Arc<SharedInner>);

#[derive(Debug)]
struct SharedInner {
  // SAFETY: Only mutating before workers can access this. Mutating is only done init-time.
  remotes: Arc<[Remote]>,
  // injector: Arc<Injector<Task>>,
}

unsafe impl Sync for SharedInner {}

impl Shared {
  /// Make sure to call [`Shared::fill_remotes`] before using Shared.
  pub fn new(workers: &[Worker]) -> Self {
    let remotes = workers
      .iter()
      .map(|worker| {
        // let stealer = worker.stealer();
        let unparker = worker.parker().unparker().clone();
        Remote::from_stealer(unparker)
      })
      .collect::<Vec<_>>();
    Self(Arc::new(SharedInner {
      remotes: Arc::from(&remotes[..]),
      // injector: Arc::new(Injector::new())
    }))
  }

  // pub fn push_task(&self, task: Task) {
  // self.0.injector.push(task);
  //
  // for remote in self.0.remotes.get().unwrap().iter() {
  //   remote.unpark();
  // }
  // }

  // pub fn iter_all_stealers(&self) -> impl Iterator<Item = &Stealer<Task>> {
  //   self.0.remotes.get().unwrap().iter().map(|remote| remote.stealer())
  // }

  // pub fn injector(&self) -> &Injector<Task> {
  //   &self.0.injector
  // }
}

use super::{worker::Worker, Remote};
use crossbeam_deque::Injector;

use crate::task::Task;

use crate::loom::sync::Arc;

#[derive(Debug)]
pub struct Shared {
  pub remotes: Box<[Remote]>,
  pub injector: Injector<Task>,
}

impl Shared {
  pub fn push_task(&self, task: Task) {
    self.injector.push(task);

    for remote in self.remotes.iter() {
      remote.unpark();
    }
  }

  pub fn from_workers(workers: &[Worker]) -> Arc<Shared> {
    let remotes = workers
      .iter()
      .map(|worker| {
        let stealer = worker.stealer();
        let unparker = worker.parker().unparker().clone();
        Remote::from_stealer(stealer, unparker)
      })
      .collect::<Vec<_>>()
      .into_boxed_slice();

    let shared = Shared { remotes, injector: Injector::new() };

    Arc::new(shared)
  }
}

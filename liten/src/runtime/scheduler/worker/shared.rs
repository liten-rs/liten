use std::sync::Arc;

use super::{worker::Worker, Remote, Workers};
use crossbeam_deque::Injector;

use crate::task::ArcTask;

pub struct Shared {
  pub remotes: Box<[Remote]>,
  pub injector: Injector<ArcTask>,
}

impl Shared {
  pub fn push_task(&self, task: ArcTask) {
    self.injector.push(task);

    for remote in self.remotes.iter() {
      remote.unpark();
    }
  }

  pub fn from_workers(
    workers: &[Worker], //num: NonZero<usize>,
                        //handle: Arc<Handle>,
  ) -> Arc<Shared> {
    let remotes = workers
      .into_iter()
      .map(|worker| {
        let stealer = worker.stealer();
        let unparker = worker.parker().unparker().clone();
        Remote::from_stealer(stealer, unparker)
      })
      .collect::<Vec<_>>()
      .into_boxed_slice();

    let shared = Shared { remotes, injector: Injector::new() };

    Arc::new(shared)

    //let num_iter = 0..num.into();

    //let workers_remotes: Vec<(Worker, /*Remote,*/ Sender<()>)> = num_iter
    //  .map(|worker_id| {
    //    let worker_queue = WorkerQueue::new_fifo();
    //    //let stealer = worker_queue.stealer();
    //    let parker = crossbeam_utils::sync::Parker::new();
    //    //let unparker = parker.unparker().clone();
    //
    //    let (sender, receiver) = oneshot::channel();
    //
    //    let worker = WorkerBuilder::with_id(worker_id)
    //      .parker(parker)
    //      .handle(handle.clone())
    //      .queue(worker_queue)
    //      .build(receiver);
    //    (worker, /*Remote::from_stealer(stealer, unparker),*/ sender)
    //  })
    //  .collect();

    //let remotes: Vec<Remote> =
    //  workers_remotes.iter().map(|(_, remote, _)| remote.clone()).collect();

    //let shutdown = ShutdownWorkers(
    //  workers_remotes
    //    .iter()
    //    .map(|(worker, remote, sender)| WorkerShutdown {
    //      worker_id: worker.id(),
    //      signal_sender: sender.clone(),
    //      unparker: remote.unparker.clone(),
    //      handle: OnceLock::new(),
    //    })
    //    .collect(),
    //);
    //
    //let workers: Vec<Worker> =
    //  workers_remotes.into_iter().map(|(worker, _)| worker).collect();
    //
    //(
    //  workers,
    //  Arc::new(Shared {
    //    remotes: remotes.into_boxed_slice(),
    //    injector: Injector::new(),
    //  }),
    //  shutdown,
    //)
  }
}

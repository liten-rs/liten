pub mod worker;

use std::{
  sync::{Arc, OnceLock},
  thread::{Builder, JoinHandle},
};

use crossbeam_deque::{Injector, Stealer, Worker as WorkerQueue};
use crossbeam_utils::sync::Unparker;
use worker::{Worker, WorkerBuilder};

use crate::{
  context,
  sync::oneshot::{self, Sender},
  task::ArcTask,
};

use super::Handle;

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

  pub fn new_parts(
    num: u8,
    handle: Arc<Handle>,
  ) -> (Vec<Worker>, Arc<Shared>, ShutdownWorkers) {
    let num_iter = 0..num as usize;

    let workers_remotes: Vec<(Worker, Remote, Sender<()>)> = num_iter
      .map(|worker_id| {
        let worker_queue = WorkerQueue::new_fifo();
        let stealer = worker_queue.stealer();
        let parker = crossbeam_utils::sync::Parker::new();
        let unparker = parker.unparker().clone();

        let (sender, receiver) = oneshot::channel();

        let worker = WorkerBuilder::with_id(worker_id)
          .parker(parker)
          .handle(handle.clone())
          .queue(worker_queue)
          .build(receiver);
        (worker, Remote::from_stealer(stealer, unparker), sender)
      })
      .collect();

    let remotes: Vec<Remote> =
      workers_remotes.iter().map(|(_, remote, _)| remote.clone()).collect();

    let shutdown = ShutdownWorkers(
      workers_remotes
        .iter()
        .map(|(worker, remote, sender)| WorkerShutdown {
          worker_id: worker.id(),
          signal_sender: sender.clone(),
          unparker: remote.unparker.clone(),
          handle: OnceLock::new(),
        })
        .collect(),
    );

    let workers: Vec<Worker> =
      workers_remotes.into_iter().map(|(worker, _, _)| worker).collect();

    (
      workers,
      Arc::new(Shared {
        remotes: remotes.into_boxed_slice(),
        injector: Injector::new(),
      }),
      shutdown,
    )
  }
}

pub struct WorkerShutdown {
  worker_id: usize,
  signal_sender: Sender<()>,
  unparker: Unparker,
  handle: OnceLock<JoinHandle<()>>,
}

pub struct ShutdownWorkers(Vec<WorkerShutdown>);

impl ShutdownWorkers {
  pub fn set_handle(&mut self, index: usize, handle: JoinHandle<()>) {
    self.0[index].handle.set(handle).unwrap();
  }
  pub fn shutdown(self) {
    for WorkerShutdown { signal_sender, unparker, handle, worker_id } in self.0
    {
      unparker.unpark();
      signal_sender.send(()).unwrap();

      handle
        .into_inner()
        .expect("worker-handle not initialied")
        .join()
        .unwrap();

      tracing::trace!(worker_id, "worker has shutdown");
    }
  }
}

// One remote worker.
#[derive(Clone)]
pub struct Remote {
  stealer: Stealer<ArcTask>,
  unparker: crossbeam_utils::sync::Unparker,
}
impl Remote {
  pub fn from_stealer(
    stealer: Stealer<ArcTask>,
    unparker: crossbeam_utils::sync::Unparker,
  ) -> Self {
    Remote { stealer, unparker }
  }

  pub fn unpark(&self) {
    self.unparker.unpark();
  }
}

//impl LaunchWorkers {
//  pub fn join(self) {
//    for handle in self.0 {
//      handle.join().unwrap();
//    }
//  }
//}

pub struct Workers(Vec<Worker>);

impl From<Vec<Worker>> for Workers {
  fn from(workers: Vec<Worker>) -> Self {
    Workers(workers)
  }
}

impl Workers {
  pub fn launch(self, handle: Arc<Handle>) -> Vec<JoinHandle<()>> {
    tracing::trace!(len = self.0.len(), "launching threads");
    let join_handles: Vec<JoinHandle<()>> = self
      .0
      .into_iter()
      .map(|mut worker| {
        let builder =
          Builder::new().name(format!("liten-worker-{}", worker.id()));
        let another_handle = handle.clone();
        builder
          .spawn(move || {
            context::runtime_enter(another_handle, move |_| worker.launch());
          })
          .unwrap()
      })
      .collect();

    join_handles
  }
}

//impl WorkersBuilder {
//  pub fn from(how_many: u8, handle: Arc<Handle>) -> Workers {
//    // Don't touch the Handle.shared
//    Workers(
//      (0..how_many as usize)
//        .into_iter()
//        .map(|index| {
//          WorkerBuilder::with_id(index)
//            .handle(handle.clone())
//            .parker(Parker::new())
//            .build()
//        })
//        .collect(),
//    )
//  }
//}

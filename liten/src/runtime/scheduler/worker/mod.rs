use std::{
  num::NonZero,
  ops::Deref,
  sync::{Arc, OnceLock},
  thread::{Builder, JoinHandle},
};

use crossbeam_deque::Stealer;
use crossbeam_utils::sync::Unparker;
use worker::Worker;

use crate::{context, sync::oneshot::Sender, task::ArcTask};

use super::Handle;

pub mod shared;
pub mod worker;

pub struct WorkerShutdown {
  worker_id: usize,
  signal_sender: Sender<()>,
  unparker: Unparker,
  handle: OnceLock<JoinHandle<()>>,
}

pub struct ShutdownWorkers(Vec<WorkerShutdown>);

impl ShutdownWorkers {
  pub fn before_starting_workers<'a>(
    workers: impl Iterator<Item = &'a Worker>,
  ) -> Self {
    ShutdownWorkers(
      workers
        .map(|x| WorkerShutdown {
          worker_id: x.id(),
          signal_sender: x.get_shutdown_sender(),
          unparker: x.parker().unparker().clone(),
          handle: OnceLock::new(),
        })
        .collect(),
    )
  }

  pub fn fill_handle(&mut self, handle: Vec<JoinHandle<()>>) {
    assert!(
      self.0.len() == handle.len(),
      "joinhandle len is not equal to workers"
    );

    for (index, handle) in handle.into_iter().enumerate() {
      self.0[index].handle.set(handle).unwrap();
    }
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

pub struct Workers(Vec<Worker>);

impl Deref for Workers {
  type Target = [Worker];

  fn deref(&self) -> &Self::Target {
    self.0.as_slice()
  }
}

impl From<Vec<Worker>> for Workers {
  fn from(workers: Vec<Worker>) -> Self {
    Workers(workers)
  }
}

impl Workers {
  pub fn new(quantity: NonZero<usize>, handle: Arc<Handle>) -> Self {
    let worker_vec: Vec<Worker> = (0..quantity.into())
      .into_iter()
      .map(|worker_id| Worker::new(worker_id, handle.clone()))
      .collect();

    Workers(worker_vec)
  }

  pub fn as_shutdown_workers(&self) -> ShutdownWorkers {
    ShutdownWorkers::before_starting_workers(self.0.iter())
  }

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

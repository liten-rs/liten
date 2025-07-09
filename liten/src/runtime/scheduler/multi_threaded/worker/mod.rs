use std::{ops::Deref, sync::OnceLock};

use crate::{
  loom::{
    sync::Arc,
    thread::{Builder, JoinHandle},
  },
  runtime::scheduler::multi_threaded::Multithreaded,
};

use parking::Unparker;
use worker::Worker;

use crate::sync::oneshot::Sender;

#[allow(clippy::module_inception)]
pub mod worker;

pub struct WorkerShutdown {
  #[allow(unused)]
  worker_id: usize,
  signal_sender: Sender<()>, // pub temp
  unparker: Unparker,
  handle: OnceLock<JoinHandle<()>>,
}

pub struct ShutdownWorkers(/* temp*/ pub Vec<WorkerShutdown>);

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
    for WorkerShutdown { signal_sender, unparker, handle, worker_id: _ } in
      self.0
    {
      signal_sender.send(()).unwrap();
      unparker.unpark();

      handle
        .into_inner()
        .expect("worker-handle not initialied")
        .join()
        .unwrap();
    }
  }
}

// One remote worker.
#[derive(Clone, Debug)]
pub struct Remote {
  // stealer: Stealer<Task>,
  unparker: Unparker,
}

impl Remote {
  #[allow(unused)]
  pub fn unpark(&self) {
    self.unparker.unpark();
  }
}
impl Deref for Workers {
  type Target = [Worker];

  fn deref(&self) -> &Self::Target {
    self.0.as_slice()
  }
}

pub struct Workers(Vec<Worker>);

impl Workers {
  pub fn new(config: Arc<Multithreaded>) -> Self {
    let worker_vec = (0..config.threads().into())
      .map(|worker_id| Worker::new(worker_id /*config.clone()*/))
      .collect();

    Workers(worker_vec)
  }

  pub fn as_shutdown_workers(&self) -> ShutdownWorkers {
    ShutdownWorkers::before_starting_workers(self.0.iter())
  }

  // FIXME: Here somewhere does the oneshot channel send so all the workers sleep.
  pub fn launch(self /*handle: Handle*/) -> Vec<JoinHandle<()>> {
    let join_handles: Vec<JoinHandle<()>> = self
      .0
      .into_iter()
      .map(|mut worker| {
        let builder =
          Builder::new().name(format!("liten-worker-{}", worker.id()));
        // let another_handle = handle.clone();
        builder
          .spawn(move || {
            worker.launch();
            // context::runtime_enter(another_handle, move |ctx| {
            //   worker.launch(ctx);
            // })
          })
          .unwrap()
      })
      .collect();

    join_handles
  }
}

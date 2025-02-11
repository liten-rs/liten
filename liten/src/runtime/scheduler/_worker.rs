//use std::{
//  collections::HashMap,
//  sync::Arc,
//  task::Poll,
//  thread::{Builder, JoinHandle, Thread},
//};
//
//use crossbeam::{
//  atomic::AtomicCell,
//  deque::{Injector, Steal, Stealer, Worker as WorkerQueue},
//};
//
//use crate::{
//  runtime::waker::LitenWaker,
//  task::{ArcTask, TaskId},
//};
//
//use super::Handle;
//
//pub struct Shared {
//  pub remotes: Box<[Remote]>,
//  pub injector: Injector<ArcTask>,
//
//  io_polling_worker: AtomicCell<Option<usize>>,
//}
//
//impl Shared {
//  pub fn push_task(&self, task: ArcTask) {
//    self.injector.push(task);
//  }
//
//  pub fn new(num: u8) -> Arc<Shared> {
//    let injector = Injector::new();
//    let num_iter = 0..num as usize;
//
//    let worker: Vec<(WorkerQueue<ArcTask>, Stealer<ArcTask>)> = num_iter
//      .map(|_| {
//        let worker_queue = WorkerQueue::new_fifo();
//        let stealer = worker_queue.stealer();
//        (worker_queue, stealer)
//      })
//      .collect();
//
//    let remotes_vec: Vec<Remote> = worker
//      .iter()
//      .map(|(_, stealer)| Remote::from_stealer(stealer.clone()))
//      .collect();
//
//    Arc::new(Shared {
//      remotes: remotes_vec.into_boxed_slice(),
//      injector,
//      io_polling_worker: AtomicCell::new(None),
//    })
//  }
//}
//
//// Local worker.
//pub struct Worker {
//  handle: Arc<Handle>,
//  local_queue: WorkerQueue<ArcTask>,
//  cold_queue: HashMap<TaskId, ArcTask>,
//  worker_id: usize,
//}
//
//impl Worker {
//  fn fetch_task(&self) -> Option<ArcTask> {
//    println!("fetchign tasks {}", self.worker_id);
//    match self.local_queue.pop() {
//      Some(value) => Some(value),
//      // Fill local queue from the global tasks
//      None => 'outer: loop {
//        match self.steal_from_global_queue() {
//          Steal::Retry => continue,
//          Steal::Success(_ /* = () */) => {
//            return Some(self.local_queue.pop().expect("what the fuck"))
//          }
//          Steal::Empty => {
//            let iter = self.handle.shared.remotes.iter().enumerate();
//            for (index, remote) in iter {
//              if index == self.worker_id {
//                continue 'outer;
//              }
//              'inner: loop {
//                match remote.stealer.steal_batch(&self.local_queue) {
//                  Steal::Retry => continue 'inner,
//                  Steal::Empty => return None,
//                  Steal::Success(_) => {
//                    return Some(self.local_queue.pop().expect(""));
//                  }
//                }
//              }
//            }
//          }
//        }
//      },
//    }
//  }
//  fn steal_from_global_queue(&self) -> Steal<()> {
//    self.handle.shared.injector.steal_batch(&self.local_queue)
//  }
//  pub fn launch(&mut self, thread_id: usize) {
//    let span = tracing::error_span!("liten-worker-", id = thread_id);
//    let _guard = span.enter();
//
//    let (sender, receiver) = crossbeam::channel::unbounded();
//    println!("bootstrapping");
//    loop {
//      for now_active_task_id in receiver.try_iter() {
//        let task = self
//          .cold_queue
//          .remove(&now_active_task_id)
//          .expect("invalid waker called, TaskId doesn't exist");
//
//        self.local_queue.push(task);
//      }
//
//      let task = match self.fetch_task() {
//        Some(task) => task,
//        None => {
//          std::thread::park();
//          continue;
//        }
//      };
//
//      let id = task.id();
//      let liten_waker = Arc::new(LitenWaker::new(id, sender.clone())).into();
//      let mut context = std::task::Context::from_waker(&liten_waker);
//
//      let unwind_task = task.clone();
//      let poll_result = match std::panic::catch_unwind(move || {
//        unwind_task.poll(&mut context)
//      }) {
//        Ok(value) => value,
//        Err(_) => todo!("handle error"),
//      };
//
//      if Poll::Pending == poll_result {
//        let old_value = self.cold_queue.insert(id, task);
//        assert!(old_value.is_none(), "logic error of inserted cold_queue task");
//      }
//    }
//  }
//}
//
//// One remote worker.
//pub struct Remote {
//  stealer: Stealer<ArcTask>,
//}
//impl Remote {
//  pub fn from_stealer(stealer: Stealer<ArcTask>) -> Self {
//    Remote { stealer }
//  }
//}
//
//pub struct WorkersBuilder;
//
//pub struct LaunchWorkers(Vec<JoinHandle<()>>);
//
//pub struct Workers(Vec<Worker>);
//
//impl Workers {
//  pub fn launch(self) -> LaunchWorkers {
//    tracing::trace!(len = self.0.len(), "launching threads");
//    let join_handles: Vec<JoinHandle<()>> = self
//      .0
//      .into_iter()
//      .enumerate()
//      .map(|(index, mut value)| {
//        let builder = Builder::new().name(format!("liten-worker-{index}"));
//
//        builder.spawn(move || value.launch(index)).unwrap()
//      })
//      .collect();
//
//    LaunchWorkers(join_handles)
//  }
//}
//
//impl WorkersBuilder {
//  pub fn from(handle: Arc<Handle>) -> Workers {
//    Workers(
//      (0..handle.shared.remotes.len())
//        .into_iter()
//        .map(|index| Worker {
//          handle: handle.clone(),
//          local_queue: WorkerQueue::new_fifo(),
//          cold_queue: HashMap::new(),
//          worker_id: index,
//        })
//        .collect(),
//    )
//  }
//}

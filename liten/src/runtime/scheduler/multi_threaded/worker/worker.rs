use crate::{
  runtime::scheduler::single_threaded::waker::create_task_waker,
  sync::oneshot::{self, OneshotError, Receiver},
  task::{Task, TaskStore},
};
use parking::Parker;

// Local worker.
pub struct Worker {
  worker_id: usize,
  // handle: Handle,
  parker: Parker,

  // hot: CBWorkerQueue<Task>,
  // cold: HashMap<TaskId, Task>,
  pub shutdown_receiver: Receiver<()>,
  // config: Arc<RuntimeBuilder>,
}

impl Worker {
  pub fn new(id: usize) -> Worker {
    let (sender, receiver) = oneshot::channel();
    drop(sender);
    Worker {
      worker_id: id,
      parker: Parker::new(),
      // hot: CBWorkerQueue::new_fifo(),
      // cold: HashMap::new(),
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

  // pub fn stealer(&self) -> Stealer<Task> {
  //   self.hot.stealer()
  // }
  //
  pub fn fetch_task(&self) -> Option<Task> {
    // if let Some(task) = self.hot.pop() {
    //   return Some(task);
    //   // Fill local queue from the global tasks
    // };

    let task_store = TaskStore::get();

    task_store.task_dequeue()
    //
    // // Try to steal tasks from the global queue
    // loop {
    //   match task_store.task_dequeue() {
    //     Some(task) => return Some(task),
    //     None => break,
    //   };
    // }

    // if self.config.enable_work_stealing {
    //   // Global queue is empty: So we steal tasks from other workers.
    //   for remote_stealer in state.iter_all_stealers() {
    //     loop {
    //       // Steal workers and pop the local queue
    //       match remote_stealer.steal_batch_and_pop(&self.hot) {
    //         // Try again with same remote
    //         Steal::Retry => continue,
    //         // Stop trying and move on to the next one.
    //         Steal::Empty => break,
    //         // Break immediately and return task
    //         Steal::Success(task) => {
    //           return Some(task);
    //         }
    //       }
    //     }
    //   }
    // }

    // None
  }

  pub fn launch(&mut self) {
    loop {
      match self.shutdown_receiver.try_recv() {
        Ok(Some(_)) => {
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

      // loop {
      //   let Ok(now_active_task_id) = receiver.try_recv() else {
      //     break;
      //   };
      //
      //   // let task = self
      //   //   .cold
      //   //   .remove(&now_active_task_id)
      //   //   .expect("invalid waker called, TaskId doesn't exist");
      //
      //   self.hot.push(task);
      // }

      let Some(task) = self.fetch_task() else {
        self.parker.park(); // this line deadlocks
        continue;
      };

      let id = task.id();
      let liten_waker = create_task_waker(self.parker.unparker(), id);
      let mut context = std::task::Context::from_waker(&liten_waker);

      let _ = std::panic::catch_unwind(move || task.poll(&mut context));
    }
  }
}

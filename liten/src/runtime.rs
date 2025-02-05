use std::{
  future::Future,
  sync::Arc,
  task::{Context as StdContext, Poll, Wake},
};

use crossbeam::channel::{self, Sender};

use crate::{context, task::Task, taskqueue::TaskQueue};

pub struct LitenWaker {
  task: Arc<Task>,
  sender: Sender<Arc<Task>>,
}

impl Wake for LitenWaker {
  fn wake(self: Arc<Self>) {
    self.sender.send(self.task.clone()).unwrap();
  }
}

pub struct Runtime {
  task_queue: TaskQueue,
  task_sender: Option<Sender<Arc<Task>>>,
}

impl Runtime {
  pub fn new() -> Self {
    Runtime { task_queue: TaskQueue::new(), task_sender: None }
  }

  fn sender(&self) -> Sender<Arc<Task>> {
    self.task_sender.clone().expect("internal 'liten' error: maintainer used Runtime.sender before setting it.")
  }

  pub fn block_on<F, Res>(&mut self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    let (sender, task_receiver) = channel::unbounded();
    self.task_sender = Some(sender);

    let _entered = context::runtime_enter(self.sender());

    let mut main_fut = Box::pin(fut);

    let (sender, receiver) = channel::unbounded();
    let waker = Arc::new(RuntimeWaker::new(sender)).into();
    let mut main_fut_context = StdContext::from_waker(&waker);

    let mut pinned = std::pin::pin!(main_fut);

    // Starts the poll so that the waker gets a change to send from the receiver.
    if let Poll::Ready(value) = pinned.as_mut().poll(&mut main_fut_context) {
      return value;
    };
    loop {
      // Fill the newest tasks onto the task queue.
      self.task_queue.take_from_iter(task_receiver.try_iter());

      if receiver.try_recv().is_ok() {
        if let Poll::Ready(value) = pinned.as_mut().poll(&mut main_fut_context)
        {
          return value;
        };
      }

      loop {
        // Sort out the tasks.
        if let Some(task) = self.task_queue.pop() {
          let waker =
            Arc::new(LitenWaker { task: task.clone(), sender: self.sender() })
              .into();
          let mut context = StdContext::from_waker(&waker);

          let task_to_send = task.clone();
          let mut task_lock = task.future.lock().unwrap();
          if task_lock.as_mut().poll(&mut context) == Poll::Pending {
            self.sender().send(task_to_send).unwrap();
          };

          if self.task_queue.is_empty() {
            break;
          }
        }
      }
    }
  }
}

// Waker implementation to notify the runtime
struct RuntimeWaker {
  sender: channel::Sender<()>,
}

impl RuntimeWaker {
  pub fn new(sender: channel::Sender<()>) -> Self {
    Self { sender }
  }
}

impl Wake for RuntimeWaker {
  fn wake(self: Arc<Self>) {
    self.sender.send(()).unwrap();
  }
}

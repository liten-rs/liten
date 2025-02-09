use std::sync::Arc;

use crossbeam::{channel::TryIter, queue::SegQueue};

use crate::task::Task;

#[derive(Debug)]
pub struct TaskQueue(SegQueue<Arc<Task>>);

impl TaskQueue {
  pub fn new() -> Self {
    Self(SegQueue::new())
  }

  //pub fn take_from_iter(&self, mut iter: TryIter<'_, Arc<Task>>) {
  //  while let Some(task) = iter.next() {
  //    self.0.push(task);
  //  }
  //}

  pub fn pop(&self) -> Option<Arc<Task>> {
    self.0.pop()
  }

  pub fn push(&self, task: Arc<Task>) {
    self.0.push(task)
  }
}

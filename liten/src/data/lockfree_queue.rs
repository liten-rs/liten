use thiserror::Error;

use crate::loom::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;

#[cfg_attr(test, derive(Debug))]
struct Cell<T> {
  sequence: AtomicUsize,
  data: UnsafeCell<MaybeUninit<T>>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct QueueBounded<T>(Arc<LFBoundedQueueInner<T>>);

#[cfg_attr(test, derive(Debug))]
struct LFBoundedQueueInner<T> {
  buffer: Box<[Cell<T>]>,
  buffer_mask: usize,
  enqueue_pos: AtomicUsize,
  dequeue_pos: AtomicUsize,
  _marker: PhantomData<T>,
}

#[derive(Error, Debug, PartialEq)]
#[error("Queue is full")]
pub struct QueueFull;

unsafe impl<T: Send> Send for QueueBounded<T> {}
unsafe impl<T: Send> Sync for QueueBounded<T> {}
unsafe impl<T: Send> Send for Cell<T> {}
unsafe impl<T: Send> Sync for Cell<T> {}

impl<T> QueueBounded<T> {
  pub fn with_capacity(capacity: usize) -> Self {
    let buffer: Vec<Cell<T>> = (0..capacity)
      .map(|i| Cell {
        sequence: AtomicUsize::new(i),
        data: UnsafeCell::new(MaybeUninit::uninit()),
      })
      .collect();

    Self(Arc::new(LFBoundedQueueInner {
      buffer: buffer.into_boxed_slice(),
      buffer_mask: capacity - 1,
      enqueue_pos: AtomicUsize::new(0),
      dequeue_pos: AtomicUsize::new(0),
      _marker: PhantomData,
    }))
  }

  pub fn push(&self, data: T) -> Result<(), QueueFull> {
    let mut pos = self.0.enqueue_pos.load(Ordering::Relaxed);
    loop {
      let cell =
        unsafe { self.0.buffer.get_unchecked(pos & self.0.buffer_mask) };
      let seq = cell.sequence.load(Ordering::Acquire);
      let dif = (seq as isize) - (pos as isize);
      if dif == 0 {
        if self
          .0
          .enqueue_pos
          .compare_exchange_weak(
            pos,
            pos + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
          )
          .is_ok()
        {
          unsafe {
            ptr::write((*cell.data.get()).as_mut_ptr(), data);
          }
          cell.sequence.store(pos + 1, Ordering::Release);
          return Ok(());
        }
      } else if dif < 0 {
        return Err(QueueFull);
      } else {
        pos = self.0.enqueue_pos.load(Ordering::Relaxed);
      }
    }
  }

  pub fn pop(&self) -> Option<T> {
    let mut pos = self.0.dequeue_pos.load(Ordering::Relaxed);
    loop {
      let cell =
        unsafe { self.0.buffer.get_unchecked(pos & self.0.buffer_mask) };
      let seq = cell.sequence.load(Ordering::Acquire);
      let dif = (seq as isize) - ((pos + 1) as isize);
      if dif == 0 {
        if self
          .0
          .dequeue_pos
          .compare_exchange_weak(
            pos,
            pos + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
          )
          .is_ok()
        {
          let data = unsafe { ptr::read((*cell.data.get()).as_ptr()) };
          cell.sequence.store(pos + self.0.buffer.len(), Ordering::Release);
          return Some(data);
        }
      } else if dif < 0 {
        return None;
      } else {
        pos = self.0.dequeue_pos.load(Ordering::Relaxed);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::loom::thread;

  #[crate::internal_test]
  fn testing() {
    let test = super::QueueBounded::<u8>::with_capacity(8);

    let test_1 = test.clone();
    let test_2 = test.clone();
    let test_3 = test.clone();
    let test_4 = test.clone();
    let test_5 = test.clone();

    [
      thread::spawn(move || test_1.push(0)),
      thread::spawn(move || test_2.push(0)),
      thread::spawn(move || test_3.push(0)),
      thread::spawn(move || test_4.push(0)),
      thread::spawn(move || test_5.push(0)),
    ]
    .into_iter()
    .for_each(|h| {
      h.join().unwrap().unwrap();
    });

    let test_1 = test.clone();
    let test_2 = test.clone();
    let test_3 = test.clone();
    let test_4 = test.clone();
    let test_5 = test.clone();

    [
      thread::spawn(move || assert!(test_1.pop().is_some_and(|t| t == 0))),
      thread::spawn(move || assert!(test_2.pop().is_some_and(|t| t == 0))),
      thread::spawn(move || assert!(test_3.pop().is_some_and(|t| t == 0))),
      thread::spawn(move || assert!(test_4.pop().is_some_and(|t| t == 0))),
      thread::spawn(move || assert!(test_5.pop().is_some_and(|t| t == 0))),
    ]
    .into_iter()
    .for_each(|h| h.join().unwrap());

    assert!(test.pop().is_none());
  }
}

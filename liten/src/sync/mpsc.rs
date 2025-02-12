use std::{
  collections::VecDeque,
  future::Future,
  sync::{
    atomic::{AtomicU16, Ordering},
    Arc, RwLock,
  },
  task::{Poll, Waker},
};

use crossbeam_utils::atomic::AtomicCell;
use futures_core::{FusedFuture, Stream};

use super::Mutex;

pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
  let channel = Arc::new(UnboundedChannel::default());
  (Sender::from(channel.clone()), Receiver::from(channel.clone()))
}

pub fn unbounded_with_capacity<T>(num: usize) -> (Sender<T>, Receiver<T>) {
  let channel = Arc::new(UnboundedChannel::with_capacity(num));
  (Sender::from(channel.clone()), Receiver::from(channel.clone()))
}

bitflags::bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  struct ChannelState: u8 {
      const INITIALISED = 0;
      const RECEIVER_DROPPED = 1 << 1;
      const WAKER_REGISTERED = 1 << 2;
  }
}

impl Default for ChannelState {
  fn default() -> Self {
    ChannelState::INITIALISED
  }
}

pub struct UnboundedChannel<T> {
  // Will always be written to so RwLock doesn't make sence.
  list: Mutex<VecDeque<T>>,
  state: AtomicCell<ChannelState>,
  num_senders: AtomicU16,
  waker: RwLock<Option<Waker>>,
}

impl<T> Default for UnboundedChannel<T> {
  fn default() -> Self {
    Self {
      list: Mutex::new(VecDeque::with_capacity(512)),
      state: AtomicCell::new(ChannelState::INITIALISED),
      num_senders: AtomicU16::new(0),
      waker: RwLock::new(None),
    }
  }
}

impl<T> UnboundedChannel<T> {
  fn with_capacity(capacity: usize) -> Self {
    Self {
      list: Mutex::new(VecDeque::with_capacity(capacity)),
      ..Default::default()
    }
  }
}

pub struct Receiver<T> {
  channel: Arc<UnboundedChannel<T>>,
}

#[derive(Debug, PartialEq)]
pub enum RecvError {
  Disconnected,
  Empty,
}

impl<T> From<Arc<UnboundedChannel<T>>> for Receiver<T> {
  fn from(channel: Arc<UnboundedChannel<T>>) -> Self {
    Self { channel }
  }
}

impl<T> Drop for Receiver<T> {
  fn drop(&mut self) {
    self
      .channel
      .state
      .fetch_update(|mut old| {
        old.insert(ChannelState::RECEIVER_DROPPED);
        Some(old)
      })
      .expect("whaat");
  }
}

pub struct ReceiverIter<'a, T>(&'a Receiver<T>);

impl<'a, T> Iterator for ReceiverIter<'a, T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    self.0.try_recv().ok()
  }
}

impl<T> Receiver<T> {
  pub fn try_iter<'a>(&'a self) -> ReceiverIter<'a, T> {
    ReceiverIter(self)
  }
}

pub struct ReceiverFuture<'a, T>(&'a Receiver<T>);

impl<T> Future for ReceiverFuture<'_, T> {
  type Output = Result<T, RecvError>;

  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    match self.0.try_recv() {
      Ok(value) => Poll::Ready(Ok(value)),
      Err(err) => match err {
        RecvError::Disconnected => Poll::Ready(Err(RecvError::Disconnected)),
        RecvError::Empty => {
          let mut lock = self.0.channel.waker.write().unwrap();
          *lock = Some(cx.waker().clone());

          Poll::Pending
        }
      },
    }
  }
}

impl<T> FusedFuture for ReceiverFuture<'_, T> {
  fn is_terminated(&self) -> bool {
    self.0.channel.state.load().contains(ChannelState::RECEIVER_DROPPED)
  }
}

impl<T> Receiver<T> {
  pub async fn recv(&self) -> Result<T, RecvError> {
    ReceiverFuture(self).await
  }

  pub fn try_recv(&self) -> Result<T, RecvError> {
    if self.channel.num_senders.load(Ordering::Relaxed) == 0 {
      return Err(RecvError::Disconnected);
    }

    let mut lock = self.channel.list.try_lock().unwrap();
    match lock.pop_front() {
      Some(t) => Ok(t),
      None => Err(RecvError::Empty),
    }
  }
}
impl<T> Stream for Receiver<T> {
  type Item = T;
  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    let pinn = std::pin::pin!(self.recv());
    match pinn.poll(cx) {
      Poll::Ready(value) => match value {
        Ok(value) => Poll::Ready(Some(value)),
        Err(err) => match err {
          RecvError::Disconnected => Poll::Ready(None),
          RecvError::Empty => Poll::Pending,
        },
      },
      Poll::Pending => Poll::Pending,
    }
  }
}

pub struct Sender<T> {
  channel: Arc<UnboundedChannel<T>>,
}

impl<T> From<Arc<UnboundedChannel<T>>> for Sender<T> {
  fn from(channel: Arc<UnboundedChannel<T>>) -> Self {
    channel.num_senders.fetch_add(1, Ordering::Relaxed);
    Self { channel }
  }
}

#[derive(Debug)]
pub struct ReceiverDroppedError;

impl<T> Sender<T> {
  pub fn send(&self, t: T) -> Result<(), ReceiverDroppedError> {
    if self.channel.num_senders.load(Ordering::Relaxed) == 0 {
      return Err(ReceiverDroppedError);
    }

    let mut lock = self.channel.list.try_lock().unwrap();
    lock.push_back(t);

    let lock = self.channel.waker.read().unwrap();

    if let Some(tesing) = lock.as_ref() {
      tesing.wake_by_ref();
    }

    Ok(())
  }
}

impl<T> Clone for Sender<T> {
  fn clone(&self) -> Self {
    self.channel.num_senders.fetch_add(1, Ordering::Relaxed);
    Sender { channel: self.channel.clone() }
  }
}

impl<T> Drop for Sender<T> {
  fn drop(&mut self) {
    self.channel.num_senders.fetch_sub(1, Ordering::Relaxed);
  }
}

#[test]
fn sender_testing() {
  let (sender, receiver) = unbounded::<i32>();

  let sender_1 = sender.clone();
  let sender_2 = sender.clone();

  sender_1.send(1).unwrap();
  sender_1.send(2).unwrap();
  sender_1.send(3).unwrap();
  assert_eq!(receiver.try_recv().unwrap(), 1);

  sender_2.send(4).unwrap();
  sender_2.send(5).unwrap();
  sender_2.send(6).unwrap();

  assert!(receiver.try_recv().unwrap() == 2);
  assert!(receiver.try_recv().unwrap() == 3);
  assert_eq!(receiver.try_recv().unwrap(), 4);
  assert!(receiver.try_recv().unwrap() == 5);
  assert!(receiver.try_recv().unwrap() == 6);
  assert!(receiver.try_recv() == Err(RecvError::Empty));
}

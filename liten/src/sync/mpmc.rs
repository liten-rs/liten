use std::{
  future::Future,
  pin::Pin,
  sync::atomic::AtomicU8,
  task::{Context, Poll, Waker},
};

use indexmap::IndexMap;

#[cfg(feature = "time")]
use crate::future::FutureExt;
#[cfg(feature = "time")]
use std::time::Duration;

use crate::{
  data::lockfree_queue::{QueueBounded, QueueFull},
  loom::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
};

pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
  let inner = Arc::new(Inner {
    queue: QueueBounded::with_capacity(capacity),
    recv_wakers: Mutex::new(IndexMap::new()),
    sender_count: AtomicUsize::new(1),
    receiver_count: AtomicUsize::new(1),
  });

  (Sender(inner.clone()), Receiver(inner))
}

struct Inner<T> {
  queue: QueueBounded<T>,
  recv_wakers: Mutex<IndexMap<usize, Waker>>,
  sender_count: AtomicUsize,
  receiver_count: AtomicUsize,
}

impl<T> Inner<T> {
  pub fn try_send(&self, item: T) -> Result<(), QueueFull> {
    // Check if all receivers have been dropped
    if self.receiver_count.load(Ordering::Acquire) == 0 {
      return Err(QueueFull);
    }
    
    // Do wakers and shit
    self.queue.push(item)?;

    if let Some((_, waker)) = self.recv_wakers.lock().unwrap().pop() {
      waker.wake();
    }

    Ok(())
  }

  pub fn try_recv(&self) -> Option<T> {
    self.queue.pop()
  }

  pub fn poll_recv(
    &self,
    future_id: usize,
    cx: &mut Context,
  ) -> Poll<Result<T, ()>> {
    match self.try_recv() {
      Some(value) => Poll::Ready(Ok(value)),
      None => {
        // Check if all senders have been dropped
        if self.sender_count.load(Ordering::Acquire) == 0 {
          return Poll::Ready(Err(()));
        }
        
        let _ = self
          .recv_wakers
          .lock()
          .unwrap()
          .insert(future_id, cx.waker().clone());
        Poll::Pending
      }
    }
  }

  pub fn increment_sender_count(&self) {
    self.sender_count.fetch_add(1, Ordering::Acquire);
  }

  pub fn decrement_sender_count(&self) {
    self.sender_count.fetch_sub(1, Ordering::Acquire);
  }

  pub fn increment_receiver_count(&self) {
    self.receiver_count.fetch_add(1, Ordering::Acquire);
  }

  pub fn decrement_receiver_count(&self) {
    self.receiver_count.fetch_sub(1, Ordering::Acquire);
  }
}

pub struct Sender<T>(Arc<Inner<T>>);

impl<T> Clone for Sender<T> {
  fn clone(&self) -> Self {
    self.0.increment_sender_count();
    Sender(self.0.clone())
  }
}

impl<T> Sender<T> {
  pub fn try_send(&self, value: T) -> Result<(), QueueFull> {
    self.0.try_send(value)
  }
}

impl<T> Drop for Sender<T> {
  fn drop(&mut self) {
    self.0.decrement_sender_count();
  }
}

pub struct Receiver<T>(Arc<Inner<T>>);

impl<T> Clone for Receiver<T> {
  fn clone(&self) -> Self {
    self.0.increment_receiver_count();
    Receiver(self.0.clone())
  }
}

impl<T> Receiver<T> {
  pub fn recv(&self) -> RecvFuture<'_, T> {
    static FUTURE_MEMORY: AtomicUsize = AtomicUsize::new(0);
    RecvFuture(self, FUTURE_MEMORY.fetch_add(1, Ordering::Acquire))
  }

  cfg_time! {
    pub async fn recv_timeout(
      &self,
      duration: Duration,
    ) -> Result<Result<T, ()>, crate::future::timeout::Timeout>
    where
      T: Send + Sync,
    {
      self.recv().timeout(duration).await
    }
  }
}

impl<T> Drop for Receiver<T> {
  fn drop(&mut self) {
    self.0.decrement_receiver_count();
  }
}

pub struct RecvFuture<'a, V>(&'a Receiver<V>, usize);

impl<V> Future for RecvFuture<'_, V> {
  type Output = Result<V, ()>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.0 .0.poll_recv(self.1, cx)
  }
}

impl<A> Drop for RecvFuture<'_, A> {
  fn drop(&mut self) {
    let _ = self.0 .0.recv_wakers.lock().unwrap().shift_remove(&self.1);
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(128);

      let result = crate::join!(
        crate::task::spawn({
          let sender = sender.clone();
          async move { sender.try_send(0u8) }
        }),
        crate::task::spawn({
          let sender = sender.clone();
          async move { sender.try_send(0u8) }
        }),
        crate::task::spawn({
          let receiver = receiver.clone();
          async move { receiver.recv().await }
        }),
        crate::task::spawn({
          let receiver = receiver.clone();
          async move { receiver.recv().await }
        }),
      );

      assert_eq!(result, (Ok(Ok(())), Ok(Ok(())), Ok(Ok(0)), Ok(Ok(0))));
    })
  }
}

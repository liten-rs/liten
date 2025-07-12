use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll, Waker},
};

#[cfg(feature = "time")]
use crate::future::FutureExt;
#[cfg(feature = "time")]
use std::time::Duration;

use indexmap::IndexMap;

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
  });

  (Sender(inner.clone()), Receiver(inner))
}

struct Inner<T> {
  queue: QueueBounded<T>,
  recv_wakers: Mutex<IndexMap<usize, Waker>>,
}

impl<T> Inner<T> {
  pub fn try_send(&self, item: T) -> Result<(), QueueFull> {
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
        let _ = self
          .recv_wakers
          .lock()
          .unwrap()
          .insert(future_id, cx.waker().clone());
        Poll::Pending
      }
    }
  }
}

#[derive(Clone)]
pub struct Sender<T>(Arc<Inner<T>>);

impl<T> Sender<T> {
  pub fn try_send(&self, value: T) -> Result<(), QueueFull> {
    self.0.try_send(value)
  }
}

pub struct Receiver<T>(Arc<Inner<T>>);

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

pub struct RecvFuture<'a, V>(&'a Receiver<V>, usize);

impl<V> Future for RecvFuture<'_, V> {
  type Output = Result<V, ()>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.0 .0.poll_recv(self.1, cx)
  }
}

impl<A> Drop for RecvFuture<'_, A> {
  fn drop(&mut self) {
    let _ = self.0 .0.recv_wakers.lock().unwrap().swap_remove(&self.1);
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
      );
      assert_eq!(result, (Ok(Ok(())), Ok(Ok(()))));

      assert_eq!(receiver.recv().await, Ok(0));
      assert_eq!(receiver.recv().await, Ok(0));
    })
  }
}

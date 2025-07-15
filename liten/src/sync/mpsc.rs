//! Multiple procuder, single consumer queue.
use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll, Waker},
};

#[cfg(feature = "time")]
use crate::future::FutureExt;
#[cfg(feature = "time")]
use std::time::Duration;

use crossbeam_queue::{ArrayQueue, SegQueue};

use crate::loom::sync::Arc;

pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
  let inner = Arc::new(Inner {
    queue: ArrayQueue::new(capacity),
    recv_wakers: SegQueue::new(),
  });

  (Sender(inner.clone()), Receiver(inner))
}

struct Inner<T> {
  queue: ArrayQueue<T>,
  recv_wakers: SegQueue<Waker>,
}

impl<T> Inner<T> {
  pub fn try_send(&self, item: T) -> Result<(), T> {
    // Do wakers and shit
    self.queue.push(item)?;

    if let Some(waker) = self.recv_wakers.pop() {
      waker.wake();
    }

    Ok(())
  }

  pub fn try_recv(&self) -> Option<T> {
    self.queue.pop()
  }

  pub fn poll_recv(&self, cx: &mut Context) -> Poll<Result<T, ()>> {
    match self.try_recv() {
      Some(value) => Poll::Ready(Ok(value)),
      None => {
        self.recv_wakers.push(cx.waker().clone());
        Poll::Pending
      }
    }
  }
}

#[derive(Clone)]
pub struct Sender<T>(Arc<Inner<T>>);

impl<T> Sender<T> {
  pub fn try_send(&self, value: T) -> Result<(), T> {
    self.0.try_send(value)
  }
}

pub struct Receiver<T>(Arc<Inner<T>>);

impl<T> Receiver<T> {
  pub fn recv(&self) -> RecvFuture<'_, T> {
    RecvFuture(self)
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

pub struct RecvFuture<'a, V>(&'a Receiver<V>);

impl<V> Future for RecvFuture<'_, V> {
  type Output = Result<V, ()>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.0 .0.poll_recv(cx)
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

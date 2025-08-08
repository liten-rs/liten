//! Multiple procuder, Multiple consumer queue.
use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll, Waker},
};

use crossbeam_queue::{ArrayQueue, SegQueue};
use thiserror::Error;

use crate::future::FutureExt;
#[cfg(feature = "time")]
use std::time::Duration;

use crate::{
  future::Stream,
  loom::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
};

pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
  let inner = Arc::new(Inner {
    queue: ArrayQueue::new(capacity),
    recv_wakers: SegQueue::new(),
    send_wakers: SegQueue::new(),
    sender_count: AtomicUsize::new(1),
    receiver_count: AtomicUsize::new(1),
  });

  (Sender(inner.clone()), Receiver(inner))
}

struct Inner<T> {
  queue: ArrayQueue<T>,
  recv_wakers: SegQueue<Waker>,
  send_wakers: SegQueue<Waker>,
  sender_count: AtomicUsize,
  receiver_count: AtomicUsize,
}

#[derive(Error, Debug, PartialEq)]
pub enum RecvError {
  #[error("Channel is closed")]
  Closed,
}

#[derive(Error, Debug, PartialEq)]
pub enum SendError<T> {
  #[error("Channel is closed")]
  Closed,

  #[error("channel is full")]
  Full(T),
}

impl<T> Inner<T> {
  pub fn try_send(&self, item: T) -> Result<(), SendError<T>> {
    // Check if all receivers have been dropped
    if self.receiver_count.load(Ordering::Acquire) == 0 {
      return Err(SendError::Closed);
    }

    // Do wakers and shit
    if let Err(value) = self.queue.push(item) {
      return Err(SendError::Full(value));
    };

    while let Some(waker) = self.recv_wakers.pop() {
      waker.wake();
    }

    Ok(())
  }

  pub fn try_recv(&self) -> Result<Option<T>, RecvError> {
    match self.queue.pop() {
      Some(value) => {
        while let Some(waker) = self.send_wakers.pop() {
          waker.wake();
        }
        Ok(Some(value))
      }
      None => {
        if self.sender_count.load(Ordering::Acquire) == 0 {
          Err(RecvError::Closed)
        } else {
          Ok(None)
        }
      }
    }
  }

  pub fn poll_recv(&self, cx: &mut Context) -> Poll<Result<T, RecvError>> {
    match self.try_recv() {
      Ok(Some(value)) => Poll::Ready(Ok(value)),
      Ok(None) => {
        self.send_wakers.push(cx.waker().clone());
        Poll::Pending
      }
      Err(err) => Poll::Ready(Err(err)),
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
  pub fn try_send(&self, value: T) -> Result<(), SendError<T>> {
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
    RecvFuture(self)
  }

  // TODO: handle queue empty and/or sender dropped.
  pub fn try_recv(&self) -> Result<Option<T>, RecvError> {
    self.0.try_recv()
  }

  cfg_time! {
    pub async fn recv_timeout(
      &self,
      duration: Duration,
    ) -> Result<Result<T, RecvError>, crate::future::timeout::Timeout>
    where
      T: Send + Sync,
    {
      self.recv().timeout(duration).await
    }
  }
}

impl<T> Stream for Receiver<T> {
  type Item = T;
  fn next(&self) -> impl Future<Output = Option<Self::Item>> {
    self.recv().map(|x| match x {
      Ok(value) => Some(value),
      Err(err) => match err {
        RecvError::Closed => None,
      },
    })
  }
}

impl<T> Drop for Receiver<T> {
  fn drop(&mut self) {
    self.0.decrement_receiver_count();
  }
}

pub struct RecvFuture<'a, V>(&'a Receiver<V>);

impl<V> Future for RecvFuture<'_, V> {
  type Output = Result<V, RecvError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.0 .0.poll_recv(cx)
  }
}

#[cfg(test)]
mod tests {
  use crate::join;
  use crate::runtime::Runtime;
  use crate::task;

  // Basic Enqueue and Dequeue Operations
  #[crate::internal_test]
  fn test_single_item_enqueue_dequeue() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(10);

      // Send a single item
      assert!(sender.try_send(42).is_ok());

      // Receive the item
      let result = receiver.recv().await;
      assert_eq!(result, Ok(42));
    });
  }

  #[crate::internal_test]
  fn test_multiple_items_fifo_order() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(10);
      let items = vec![1, 2, 3, 4, 5];

      // Send multiple items
      for (index, &item) in items.iter().enumerate() {
        println!("{}", index);
        assert_eq!(sender.try_send(item), Ok(()));
      }

      // Receive items in FIFO order
      for &expected in &items {
        let result = receiver.recv().await;
        assert_eq!(result, Ok(expected));
      }
    });
  }

  // Empty Queue Behavior
  #[crate::internal_test]
  fn test_empty_queue_try_recv() {
    Runtime::single_threaded().block_on(async {
      let (_s, receiver) = super::bounded::<i32>(10);

      // Try to receive from empty queue
      let result = receiver.try_recv();
      assert_eq!(result, Ok(None));
    });
  }

  #[crate::internal_test]
  fn test_empty_queue_recv_blocks() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded::<i32>(10);

      // Start a receiver that should block initially
      let recv_handle = task::spawn(async move { receiver.recv().await });

      // Send an item to unblock the receiver
      sender.try_send(42).unwrap();

      // Now the receiver should complete
      let result = recv_handle.await.unwrap();
      assert_eq!(result, Ok(42));
    });
  }

  // Full Queue Behavior
  #[crate::internal_test]
  fn test_full_queue_behavior() {
    Runtime::single_threaded().block_on(async {
      let (sender, _r) = super::bounded(2);

      // Fill the queue
      assert!(sender.try_send(1).is_ok());
      assert!(sender.try_send(2).is_ok());

      // Try to send to full queue
      let result = sender.try_send(3);
      assert_eq!(result, Err(super::SendError::Full(3)));
    });
  }

  // Performance Tests
  #[crate::internal_test]
  #[cfg(not(miri))]
  fn test_throughput_measurement() {
    Runtime::single_threaded().block_on(async {
      use std::time::Instant;

      let (sender, receiver) = super::bounded(100000);
      let num_items = 100_000;
      let num_producers = 4;
      let num_consumers = 4;

      let start = Instant::now();

      // Spawn producers
      let producer_handles: Vec<_> = (0..num_producers)
        .map(|_| {
          let sender = sender.clone();
          task::spawn(async move {
            for i in 0..(num_items / num_producers) {
              sender.try_send(i).unwrap();
            }
          })
        })
        .collect();

      // Spawn consumers
      let consumer_handles: Vec<_> = (0..num_consumers)
        .map(|_| {
          let receiver = receiver.clone();
          task::spawn(async move {
            for _ in 0..(num_items / num_consumers) {
              receiver.recv().await.unwrap();
            }
          })
        })
        .collect();

      // Wait for completion
      for handle in producer_handles {
        handle.await.unwrap();
      }
      for handle in consumer_handles {
        handle.await.unwrap();
      }

      let duration = start.elapsed();
      let throughput = num_items as f64 / duration.as_secs_f64();

      println!("Throughput: {:.2} items/sec", throughput);
      assert!(throughput > 1000.0); // Should handle at least 1000 items/sec
    });
  }

  // Stress Tests

  #[crate::internal_test]
  fn test_long_running_continuous_operation() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(100);

      // Spawn continuous producer
      let producer_handle = task::spawn(async move {
        let mut counter = 0;
        if sender.try_send(counter).is_ok() {
          counter += 1;
        }
        counter
      });

      // Spawn continuous consumer
      let consumer_handle = task::spawn(async move {
        let mut received = 0;
        if let Ok(_) = receiver.recv().await {
          received += 1;
        }
        received
      });

      let (sent_count, received_count) =
        join!(producer_handle, consumer_handle);
      let sent_count = sent_count.unwrap();
      let received_count = received_count.unwrap();

      println!("Sent: {}, Received: {}", sent_count, received_count);
      assert!(sent_count > 0);
      assert!(received_count > 0);
      assert!(received_count == sent_count); // At least 80% should be received
    });
  }

  // Edge Cases
  #[crate::internal_test]
  fn test_channel_closed_when_all_senders_dropped() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded::<i32>(10);

      // Send some items
      sender.try_send(1).unwrap();
      sender.try_send(2).unwrap();

      // Drop the sender
      drop(sender);

      // Should still be able to receive remaining items
      assert_eq!(receiver.recv().await, Ok(1));
      assert_eq!(receiver.recv().await, Ok(2));

      // Now should get closed error
      assert_eq!(receiver.recv().await, Err(super::RecvError::Closed));
    });
  }

  #[crate::internal_test]
  fn test_channel_closed_when_all_receivers_dropped() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded::<i32>(10);

      // Drop the receiver
      drop(receiver);

      // Sender should get closed error
      assert_eq!(sender.try_send(42), Err(super::SendError::Closed));
    });
  }

  #[crate::internal_test]
  fn test_multiple_senders_and_receivers() {
    Runtime::single_threaded().block_on(async {
      let (sender1, receiver1) = super::bounded(10);
      let sender2 = sender1.clone();
      let receiver2 = receiver1.clone();

      // Send from both senders
      sender1.try_send(1).unwrap();
      sender2.try_send(2).unwrap();

      // Receive from both receivers
      let result1 = receiver1.recv().await;
      let result2 = receiver2.recv().await;

      // Both should receive items (order may vary)
      assert!(result1.is_ok());
      assert!(result2.is_ok());
      assert_ne!(result1.unwrap(), result2.unwrap());
    });
  }

  // Thread Safety Tests
  #[crate::internal_test]
  fn test_concurrent_clone_and_drop() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(100);
      let num_operations = 1000;

      // Spawn tasks that clone and drop senders/receivers
      let sender_handles: Vec<_> = (0..4)
        .map(|_| {
          let sender = sender.clone();
          task::spawn(async move {
            for _ in 0..num_operations {
              let _cloned_sender = sender.clone();
              // Drop happens automatically
            }
          })
        })
        .collect();

      let receiver_handles: Vec<_> = (0..4)
        .map(|_| {
          let receiver = receiver.clone();
          task::spawn(async move {
            for _ in 0..num_operations {
              let _cloned_receiver = receiver.clone();
              // Drop happens automatically
            }
          })
        })
        .collect();

      // Wait for all operations
      for handle in sender_handles {
        handle.await.unwrap();
      }
      for handle in receiver_handles {
        handle.await.unwrap();
      }

      // Channel should still work
      sender.try_send(42).unwrap();
      assert_eq!(receiver.recv().await, Ok(42));
    });
  }

  // Timeout Tests (if time feature is enabled)
  #[cfg(feature = "time")]
  #[crate::internal_test]
  fn test_recv_timeout() {
    use std::time::Duration;

    Runtime::single_threaded().block_on(async {
      use crate::future::timeout::Timeout;

      let (_send, receiver) = super::bounded::<i32>(10);

      // Try to receive with timeout
      let result = receiver.recv_timeout(Duration::from_millis(15)).await;
      assert_eq!(result, Err(Timeout)); // Should timeout
    });
  }

  #[cfg(feature = "time")]
  #[crate::internal_test]
  fn test_recv_timeout_with_data() {
    use std::time::Duration;

    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(10);

      // Send data
      sender.try_send(42).unwrap();

      // Try to receive with timeout
      let result = receiver.recv_timeout(Duration::from_millis(100)).await;
      assert_eq!(result, Ok(Ok(42))); // Should receive data
    });
  }

  // Original test for backward compatibility
  #[crate::internal_test]
  fn test_basic_concurrent_operations() {
    Runtime::single_threaded().block_on(async {
      let (sender, receiver) = super::bounded(128);

      let result = join!(
        task::spawn({
          let sender = sender.clone();
          async move { sender.try_send(0u8) }
        }),
        task::spawn({
          let sender = sender.clone();
          async move { sender.try_send(0u8) }
        }),
        task::spawn({
          let receiver = receiver.clone();
          async move { receiver.recv().await }
        }),
        task::spawn({
          let receiver = receiver.clone();
          async move { receiver.recv().await }
        }),
      );

      assert_eq!(result, (Ok(Ok(())), Ok(Ok(())), Ok(Ok(0)), Ok(Ok(0))));
    });
  }
}

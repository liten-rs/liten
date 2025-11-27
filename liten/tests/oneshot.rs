#![cfg(feature = "sync")]

use std::future::Future;

use liten::sync::oneshot::{self};

macro_rules! get_ready {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!($expr);
    match pinned.as_mut().poll(&mut std::task::Context::from_waker(
      &liten::testing_util::noop_waker(),
    )) {
      std::task::Poll::Ready(value) => value,
      std::task::Poll::Pending => unreachable!("was Poll::Pending"),
    }
  }};
}

const VALUE: u8 = 42;

// ===== Basic Channel Tests =====
#[liten::internal_test]
fn happy_path() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Send should succeed and mem::forget the sender
  assert_eq!(sender.send(VALUE), Ok(()));

  // The sender should not be dropped (due to mem::forget)
  // but the value should still be received
  assert_eq!(get_ready!(receiver), Ok(VALUE));
}

// ===== Drop Handling Tests =====

#[liten::internal_test]
fn sender_dropped_before_send() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = get_ready!(receiver);
  assert_eq!(result, Err(oneshot::OneshotError::SenderDropped));
}

#[liten::internal_test]
fn receiver_dropped_before_send() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(receiver);

  let result = sender.send(VALUE);
  assert_eq!(result, Err(oneshot::OneshotError::ReceiverDropped));
}

// ===== Try Recv Tests =====

#[liten::internal_test]
fn try_recv() {
  let (sender, receiver) = oneshot::channel::<u8>();

  let result = receiver.try_recv().unwrap();
  assert_eq!(result, None);

  // Send the value
  sender.send(VALUE).unwrap();

  // Now try_recv should succeed
  assert_eq!(receiver.try_recv().unwrap(), Some(VALUE));

  // Should now fail
  assert_eq!(
    receiver.try_recv(),
    Err(oneshot::OneshotError::RecvAfterTakenValue)
  );
}

#[liten::internal_test]
#[cfg(feature = "runtime")]
fn channel_try_recv_sender_dropped() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = receiver.try_recv();
  assert_eq!(result, Err(oneshot::OneshotError::SenderDropped));

  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = liten::block_on(receiver);
  assert_eq!(result, Err(oneshot::OneshotError::SenderDropped));
}

// ===== Complex Scenarios =====

#[liten::internal_test]
fn complex_receiver_future() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Start receiving (should be pending)
  let mut recv_future = std::pin::pin!(receiver);
  let poll_result = recv_future.as_mut().poll(
    &mut std::task::Context::from_waker(&liten::testing_util::noop_waker()),
  );
  assert!(matches!(poll_result, std::task::Poll::Pending));

  // Now send
  sender.send(VALUE).unwrap();

  // Should be ready now
  let result = get_ready!(recv_future);
  assert_eq!(result, Ok(VALUE));
}

// ===== Waker Behavior Tests =====

#[liten::internal_test]
fn channel_waker_wake_on_send() {
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::task::{RawWaker, RawWakerVTable, Waker};

  let (sender, receiver) = oneshot::channel::<u8>();

  static CALLED: AtomicBool = AtomicBool::new(false);
  fn clone(_: *const ()) -> RawWaker {
    raw_waker()
  }
  fn wake(_: *const ()) {
    CALLED.store(true, Ordering::SeqCst);
  }
  fn wake_by_ref(_: *const ()) {
    CALLED.store(true, Ordering::SeqCst);
  }
  fn drop(_: *const ()) {}
  fn raw_waker() -> RawWaker {
    RawWaker::new(
      std::ptr::null(),
      &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
    )
  }
  let waker = unsafe { Waker::from_raw(raw_waker()) };

  // Start receiving (should be pending)
  let mut recv_future = std::pin::pin!(receiver);
  let mut cx = std::task::Context::from_waker(&waker);
  let poll_result = recv_future.as_mut().poll(&mut cx);
  assert!(matches!(poll_result, std::task::Poll::Pending));

  // Send should wake the receiver
  sender.send(VALUE).unwrap();

  // Waker should have been called
  assert!(CALLED.load(Ordering::SeqCst));

  // Should be ready now
  let result = get_ready!(recv_future);
  assert_eq!(result, Ok(VALUE));
}

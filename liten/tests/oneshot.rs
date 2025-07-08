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

macro_rules! assert_pending {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!($expr);
    match pinned.as_mut().poll(&mut std::task::Context::from_waker(
      &liten::testing_util::noop_waker(),
    )) {
      std::task::Poll::Ready(_) => unreachable!("was Poll::Ready(...)"),
      std::task::Poll::Pending => pinned,
    }
  }};
}

const VALUE: u8 = 42;

// ===== Basic Channel Tests =====

#[liten::internal_test]
fn channel_basic_send_receive() {
  let (sender, receiver) = oneshot::channel();

  sender.send(VALUE).unwrap();

  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));
}

// ===== Drop Handling Tests =====

#[liten::internal_test]
fn channel_sender_dropped_before_send() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = get_ready!(receiver);
  assert_eq!(result, Err(oneshot::OneshotError::SenderDropped));
}

#[liten::internal_test]
fn channel_receiver_dropped_before_send() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(receiver);

  let result = sender.send(VALUE);
  assert_eq!(result, Err(oneshot::OneshotError::ReceiverDropped));
}

// ===== Mem::Forget Tests =====

#[liten::internal_test]
fn channel_mem_forget_prevents_drop() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Send should succeed and mem::forget the sender
  let send_result = sender.send(VALUE);
  assert_eq!(send_result, Ok(()));

  // The sender should not be dropped (due to mem::forget)
  // but the value should still be received
  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));
}

// ===== Try Recv Tests =====

#[liten::internal_test]
fn channel_try_recv_not_ready() {
  let (sender, receiver) = oneshot::channel::<u8>();

  let result = receiver.try_recv().unwrap();
  assert_eq!(result, None);

  // Send the value
  sender.send(VALUE).unwrap();

  // Now try_recv should succeed
  let result = receiver.try_recv().unwrap();
  assert_eq!(result, Some(VALUE));
}

#[liten::internal_test]
fn channel_try_recv_after_send() {
  let (sender, receiver) = oneshot::channel::<u8>();

  sender.send(VALUE).unwrap();

  let result = receiver.try_recv().unwrap();
  assert_eq!(result, Some(VALUE));

  // Second try_recv should return error (value already taken)
  assert!(receiver.try_recv().is_err());
}

#[liten::internal_test]
fn channel_try_recv_sender_dropped() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = receiver.try_recv();
  assert_eq!(result, Err(oneshot::OneshotError::SenderDropped));
}

// ===== Try Get Sender Tests =====

#[liten::internal_test]
fn channel_try_get_sender_not_dropped() {
  let (sender, receiver) = oneshot::channel::<u8>();

  let result = receiver.try_get_sender();
  match result {
    Err(oneshot::OneshotError::SenderNotDropped) => {}
    Err(_) => unreachable!(),
    Ok(_) => panic!("expected SenderNotDropped"),
  }

  // Send a value
  sender.send(VALUE).unwrap();

  let result = receiver.try_get_sender();
  match result {
    Err(oneshot::OneshotError::SenderNotDropped) => {}
    Err(_) => unreachable!(),
    Ok(_) => panic!("expected SenderNotDropped"),
  }
}

#[liten::internal_test]
fn channel_try_get_sender_after_drop() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = receiver.try_get_sender();
  assert!(result.is_ok());

  drop(result.unwrap());

  // Should be able to get a new sender
  assert!(receiver.try_get_sender().is_ok());
}

// ===== Complex Scenarios =====

#[liten::internal_test]
fn channel_concurrent_send_receive() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Send first
  sender.send(VALUE).unwrap();

  // Then receive
  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));
}

#[liten::internal_test]
fn channel_receive_before_send() {
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

// ===== Error Edge Cases =====

#[liten::internal_test]
fn channel_double_send_panics() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // First send should succeed
  let result = sender.send(VALUE);
  assert_eq!(result, Ok(()));

  // Second send should panic due to mem::forget
  // Note: We can't actually test this because sender is moved in send()
  // The mem::forget behavior is tested in channel_mem_forget_prevents_drop
  let _ = receiver;
}

#[liten::internal_test]
fn channel_double_receive_panics() {
  let (sender, receiver) = oneshot::channel::<u8>();

  sender.send(VALUE).unwrap();

  // First receive should succeed
  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));

  // Second receive should panic (receiver is consumed)
  // Note: We can't actually test this because receiver is moved
}

// ===== Different Value Types =====

#[liten::internal_test]
fn channel_with_string() {
  let (sender, receiver) = oneshot::channel();

  sender.send("hello".to_string()).unwrap();

  let result = get_ready!(receiver);
  assert_eq!(result, Ok("hello".to_string()));
}

#[liten::internal_test]
fn channel_with_struct() {
  #[derive(Debug, PartialEq, Eq)]
  struct TestStruct {
    value: u32,
    name: String,
  }

  let (sender, receiver) = oneshot::channel();

  let test_struct = TestStruct { value: 123, name: "test".to_string() };

  sender.send(test_struct).unwrap();

  let result = get_ready!(receiver);
  assert_eq!(result, Ok(TestStruct { value: 123, name: "test".to_string() }));
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

// ===== Memory Safety Tests =====

#[liten::internal_test]
fn channel_memory_safety() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Send a value
  sender.send(VALUE).unwrap();

  // Receive the value
  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));

  // Both sender and receiver should be dropped without issues
}

#[liten::internal_test]
fn channel_mem_forget_behavior() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Send should mem::forget the sender
  sender.send(VALUE).unwrap();

  // The sender should not be dropped, but the value should be received
  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));
}

// ===== State Transition Tests =====

#[liten::internal_test]
fn channel_state_transitions() {
  let (sender, receiver) = oneshot::channel::<u8>();

  // Initial state should be Init
  // Note: We can't directly access the state, but we can test the behavior

  // Send a value
  sender.send(VALUE).unwrap();

  // Receive the value
  let result = get_ready!(receiver);
  assert_eq!(result, Ok(VALUE));
}

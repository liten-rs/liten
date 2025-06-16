use std::future::Future;

use liten::sync::oneshot::{self, sync::OneshotError};

macro_rules! get_ready {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!($expr);
    match pinned
      .as_mut()
      .poll(&mut std::task::Context::from_waker(&futures_task::noop_waker()))
    {
      std::task::Poll::Ready(value) => value,
      std::task::Poll::Pending => unreachable!("was Poll::Pending"),
    }
  }};
}

macro_rules! should_pending {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!(&mut $expr);
    match pinned
      .as_mut()
      .poll(&mut std::task::Context::from_waker(&futures_task::noop_waker()))
    {
      std::task::Poll::Ready(_) => false,
      std::task::Poll::Pending => true,
    }
  }};
}

const VALUE: u8 = 2;

#[liten::internal_test]
fn non_sync_channel() {
  let (sender, receiver) = oneshot::channel();

  sender.send(VALUE).unwrap();

  let result = get_ready!(receiver);

  assert_eq!(result, Ok(VALUE));
}

#[liten::internal_test]
fn non_sync_basic_drop_handling() {
  let (sender, receiver) = oneshot::channel::<u8>();

  drop(sender);

  let result = get_ready!(receiver);

  assert_eq!(result, Err(oneshot::not_sync::OneshotError::SenderDropped));

  let (sender, receiver) = oneshot::channel::<u8>();

  drop(receiver);

  let result = sender.send(VALUE);

  assert_eq!(result, Err(oneshot::not_sync::OneshotError::ReceiverDropped));
}

#[liten::internal_test]
fn sync_drop_handling() {
  let (sender, receiver) = oneshot::sync_channel::<u8>();
  drop(sender);

  let should_err = get_ready!(receiver);

  assert_eq!(should_err, Err(OneshotError::ChannelDropped));

  let (sender, receiver) = oneshot::sync_channel::<u8>();
  drop(receiver);

  let should_err = get_ready!(sender.send(VALUE));

  assert_eq!(should_err, Err(OneshotError::ChannelDropped));
}

#[liten::internal_test]
fn sync_happy_path() {
  let (sender, mut receiver) = oneshot::sync_channel::<u8>();

  assert!(should_pending!(receiver));

  let mut fut = sender.send(VALUE);

  get_ready!(fut).unwrap();

  let result = get_ready!(receiver);

  assert_eq!(result, Ok(VALUE));
}

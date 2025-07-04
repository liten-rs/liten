use std::future::Future;

use liten::sync::oneshot::{self};

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

  assert_eq!(result, Err(oneshot::OneshotError::SenderDropped));

  let (sender, receiver) = oneshot::channel::<u8>();

  drop(receiver);

  let result = sender.send(VALUE);

  assert_eq!(result, Err(oneshot::OneshotError::ReceiverDropped));
}

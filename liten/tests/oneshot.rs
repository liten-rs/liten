use std::future::Future;
use std::task::Context;

use liten::sync::oneshot::sync::OneshotError;

use futures_task::noop_waker;

macro_rules! get_ready {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!($expr);
    match pinned.as_mut().poll(&mut Context::from_waker(&noop_waker())) {
      std::task::Poll::Ready(value) => value,
      std::task::Poll::Pending => unreachable!("was Poll::Pending"),
    }
  }};
}

#[test]
fn non_sync_channel() {
  liten::runtime::Runtime::new().block_on(async {
    let (sender, receiver) = liten::sync::oneshot::channel();

    const VALUE: u8 = 2;

    sender.send(VALUE).unwrap();

    let result = get_ready!(receiver);

    assert_eq!(result, Ok(2));
  });
}

#[test]
fn sync_channel_drop_handling() {
  liten::runtime::Runtime::new().block_on(async {
    let (sender, receiver) = liten::sync::oneshot::sync_channel::<u8>();
    drop(sender);

    let should_err = get_ready!(receiver);

    assert_eq!(should_err, Err(OneshotError::ChannelDropped));

    let (sender, receiver) = liten::sync::oneshot::sync_channel::<u8>();
    drop(receiver);

    let should_err = get_ready!(sender.send(0));

    assert_eq!(should_err, Err(OneshotError::ChannelDropped));
  });
}

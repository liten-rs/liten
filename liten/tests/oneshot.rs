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

macro_rules! should_pending {
  ($expr:expr) => {{
    let mut pinned = std::pin::pin!(&mut $expr);
    match pinned.as_mut().poll(&mut Context::from_waker(&noop_waker())) {
      std::task::Poll::Ready(_) => false,
      std::task::Poll::Pending => true,
    }
  }};
}

#[cfg(loom)]
#[test]
fn non_sync_channel() {
  loom::model(|| {
    let (sender, receiver) = liten::sync::oneshot::channel();

    const VALUE: u8 = 2;

    sender.send(VALUE).unwrap();

    let result = get_ready!(receiver);

    assert_eq!(result, Ok(2));
  })
}

#[cfg(loom)]
#[test]
fn sync_drop_handling() {
  loom::model(|| {
    let (sender, receiver) = liten::sync::oneshot::sync_channel::<u8>();
    drop(sender);

    let should_err = get_ready!(receiver);

    assert_eq!(should_err, Err(OneshotError::ChannelDropped));

    let (sender, receiver) = liten::sync::oneshot::sync_channel::<u8>();
    drop(receiver);

    let should_err = get_ready!(sender.send(0));

    assert_eq!(should_err, Err(OneshotError::ChannelDropped));
  })
}

#[cfg(loom)]
#[test]
fn sync_happy_path() {
  loom::model(|| {
    let (sender, mut receiver) = liten::sync::oneshot::sync_channel::<u8>();

    assert!(should_pending!(receiver));

    let mut fut = sender.send(0);

    get_ready!(fut).unwrap();

    let result = get_ready!(receiver);

    assert_eq!(result, Ok(0));
  });
}

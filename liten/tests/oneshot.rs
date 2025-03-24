use std::future::Future;
use std::task::Context;

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

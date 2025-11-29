#![cfg(linux)]
#![cfg(feature = "high")]
/// write in append mode is not tested since `pwrite` doesn't support it.
use std::{future::Future, pin::Pin, task::Context, time::Duration};

use futures_task::noop_waker;

#[test]
fn test_timeout() {
  liten::block_on(async {
    println!("testing");

    let mut _test = lio::timeout(Duration::from_millis(1000));

    let _ = Pin::new(&mut _test).poll(&mut Context::from_waker(&noop_waker()));

    std::thread::sleep(Duration::from_secs(3));
    drop(_test);
  });
}

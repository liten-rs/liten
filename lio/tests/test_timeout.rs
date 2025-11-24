#![cfg(linux)]
/// write in append mode is not tested since `pwrite` doesn't support it.
use std::{
  future::Future,
  pin::Pin,
  task::Context,
  time::{Duration, Instant},
};

use futures_task::noop_waker;
use libc::wait;

#[test]
fn test_timeout() {
  liten::block_on(async {
    println!("testing");
    // let nice = Instant::now();
    // let _ = lio::timeout(Duration::from_millis(1000)).await.unwrap();
    // dbg!(nice.elapsed());

    let mut _test = lio::timeout(Duration::from_millis(1000));

    let result =
      Pin::new(&mut _test).poll(&mut Context::from_waker(&noop_waker()));

    std::thread::sleep(Duration::from_secs(3));
    drop(_test);

    // let result =
    //   Pin::new(&mut _test).poll(&mut Context::from_waker(&noop_waker()));
  });
}

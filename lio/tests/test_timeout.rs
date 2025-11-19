#![cfg(linux)]
/// write in append mode is not tested since `pwrite` doesn't support it.
use std::time::{Duration, Instant};

#[test]
fn test_timeout() {
  liten::block_on(async {
    println!("testing");
    let nice = Instant::now();
    let _ = lio::timeout(Duration::from_millis(1000)).await.unwrap();
    dbg!(nice.elapsed());
  });
}

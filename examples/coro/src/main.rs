use std::time::Duration;

use liten::coro::*;
use liten::future::FutureExt;

fn main() {
  let _handle = init();
  let testing = async { 0 }.spawn();
  let testing2 = liten::time::sleep(Duration::from_millis(1000)).spawn();

  dbg!(testing.join());
  dbg!(testing2.join());
}

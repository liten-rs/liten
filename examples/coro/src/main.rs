use std::time::Duration;

use liten::coro::go;

fn main() {
  let testing2 = go(async {
    let testing = go(async {
      liten::time::sleep(Duration::from_millis(1500)).await;
      dbg!("yes");
    });
    liten::time::sleep(Duration::from_millis(1000)).await;

    dbg!("yes");

    testing.await;
  });

  std::thread::sleep(Duration::from_secs(2));
  // // dbg!("yes");
  dbg!(testing2.join());
  // dbg!(testing.join());
  liten::coro::shutdown();
}

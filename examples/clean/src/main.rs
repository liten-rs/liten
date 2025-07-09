use std::{thread, time::Duration};

use liten::{blocking::unblock, task};

#[liten::main]
async fn main() {
  task::spawn(async {});
  unblock(|| thread::sleep(Duration::from_millis(500))).await;
}

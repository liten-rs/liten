use std::{error::Error, future::ready};

use liten::task;

fn main() -> Result<(), Box<dyn Error>> {
  let task = task::spawn(ready(1));

  assert!(task.rejoin()? == 1);
  Ok(())
}

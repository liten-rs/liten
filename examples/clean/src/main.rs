use std::{error::Error, thread::sleep, time::Duration};

use tracing::{Level, subscriber};
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
  subscriber::set_global_default(fmt().with_max_level(Level::TRACE).finish())?;
  liten_new::runtime::Runtime::builder().block_on(async {
    liten_new::task::spawn(async {
      tracing::info!("Very nice");
    });
    println!("program stop -----");
  });

  liten_new::runtime::Runtime::builder().block_on(async {
    liten_new::task::spawn(async {
      tracing::info!("Very nice");
    });
    println!("program stop -----");
    Ok(())
  })
}

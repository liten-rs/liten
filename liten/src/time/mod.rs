mod clock;
mod sleep;
mod utils;
mod wheel;
use std::{
  task::Waker,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clock::Clock;
pub use sleep::*;

#[derive(Debug)]
pub struct TimeDriver {
  clock: Clock,

  last_advance: Instant,
  // nearest_timer: Instant,
}

pub enum TimeDriverInput {}

impl TimeDriver {
  pub fn new() -> Self {
    let (days, hours, minutes, seconds, milliseconds) =
      utils::breakdown_milliseconds(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
          as usize,
      );
    Self {
      clock: Clock::new_with_positions(
        days,
        hours,
        minutes,
        seconds,
        milliseconds,
      ),
      last_advance: Instant::now(),
    }
  }

  fn jump(&mut self) {
    self.clock.advance(self.last_advance.elapsed().as_millis() as usize);
    self.last_advance = Instant::now();
  }
}

#[test]
fn timer_playground() {
  let mut mng = TimeDriver::new();

  std::thread::sleep(Duration::from_millis(1));

  mng.jump();
}

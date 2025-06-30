use std::{task::Waker, time::Instant};

use crate::time::{utils, wheel::TimerTickResult};

use super::wheel::Wheel;

#[derive(Debug, Clone)]
pub struct Timer {
  waker: Waker,

  /// duration from start instant to when timer is actuated.
  timestamp: usize,
}

impl Timer {
  fn new(waker: Waker, timestamp: usize) -> Self {
    Self { waker, timestamp }
  }

  fn timestamp(&self) -> usize {
    self.timestamp
  }
  fn trigger(self) {
    println!("triggered");
    self.waker.wake();
    // Actuate the waker
  }
}

#[derive(Debug)]
pub struct Clock {
  milliseconds: Wheel<1000, Timer>,
  seconds: Wheel<60, Timer>,
  minutes: Wheel<60, Timer>,
  hours: Wheel<24, Timer>,
  days: Wheel<365, Timer>,

  start_instant: Instant,
}

impl Clock {
  pub fn new() -> Self {
    Self::new_with_positions(0, 0, 0, 0, 0)
  }

  pub fn new_with_positions(
    days: usize,
    hours: usize,
    minutes: usize,
    seconds: usize,
    milliseconds: usize,
  ) -> Self {
    Self {
      milliseconds: Wheel::new_with_position(milliseconds as usize),
      seconds: Wheel::new_with_position(seconds as usize),
      minutes: Wheel::new_with_position(minutes as usize),
      hours: Wheel::new_with_position(hours as usize),
      days: Wheel::new_with_position(days as usize),

      start_instant: Instant::now(),
    }
  }

  // TODO
  pub fn peek_nearest_timer(&mut self) -> Option<usize> {
    self
      .milliseconds
      .peak_nearest()
      .or(self.seconds.peak_nearest())
      .or(self.minutes.peak_nearest())
      .or(self.hours.peak_nearest())
      .or(self.days.peak_nearest())
  }

  pub fn start_elapsed(&self) -> usize {
    self.start_instant.elapsed().as_millis() as usize
  }

  pub fn insert(&mut self, millis: usize, timer: Timer) {
    let (day_ticks, hour_ticks, minute_ticks, second_ticks, millisecond_ticks) =
      utils::breakdown_milliseconds(millis);

    assert_eq!(day_ticks, 0, "If downgrading days cannot be non-0");

    if hour_ticks != 0 {
      self.hours.insert(hour_ticks, timer);
    } else if minute_ticks != 0 {
      self.minutes.insert(minute_ticks, timer);
    } else if second_ticks != 0 {
      self.seconds.insert(second_ticks, timer);
    } else if millisecond_ticks != 0 {
      self.milliseconds.insert(millisecond_ticks, timer);
    } else {
      panic!("Time cannot be 00:00:00:00")
    }
  }

  pub fn advance(&mut self, millis: usize) -> Vec<Timer> {
    let (day_ticks, hour_ticks, minute_ticks, second_ticks, millisecond_ticks) =
      utils::breakdown_milliseconds(millis);

    let mut vec = Vec::new();

    let TimerTickResult { slots, resetted_counter } =
      self.milliseconds.advance(millisecond_ticks);
    vec.extend(slots);

    let TimerTickResult { slots, resetted_counter } =
      self.seconds.advance(second_ticks + resetted_counter);
    vec.extend(slots);

    let TimerTickResult { slots, resetted_counter } =
      self.minutes.advance(minute_ticks + resetted_counter);
    vec.extend(slots);

    let TimerTickResult { slots, resetted_counter } =
      self.hours.advance(hour_ticks + resetted_counter);
    vec.extend(slots);

    let TimerTickResult { slots, resetted_counter } =
      self.days.advance(day_ticks + resetted_counter);
    vec.extend(slots);

    assert_eq!(resetted_counter, 0, "what to do?");

    vec
      .into_iter()
      .filter_map(|item| {
        let elapsed = self.start_elapsed();
        if elapsed <= item.timestamp() {
          return Some(item);
        }

        let delta_now_to_timestamp = elapsed - item.timestamp();
        self.insert(delta_now_to_timestamp, item);

        return None;
      })
      .collect()
  }
}

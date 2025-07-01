use std::{
  sync::atomic::{AtomicUsize, Ordering},
  time::Instant,
};

use crate::time::{utils, wheel::TimerTickResult};

use super::wheel::Wheel;

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct TimerId {
  id: usize,
  timestamp: usize,
}

impl TimerId {
  pub(in crate::time) fn new(timestamp: usize) -> Self {
    static TIMER_ID: AtomicUsize = AtomicUsize::new(0);

    Self { timestamp, id: TIMER_ID.fetch_add(1, Ordering::AcqRel) }
  }
  pub fn id(&self) -> usize {
    self.id
  }

  fn timestamp(&self) -> usize {
    self.timestamp
  }

  pub fn has_happened(&self, timestamp: usize) -> bool {
    self.timestamp < timestamp
  }
}

#[derive(Debug)]
pub struct Clock {
  milliseconds: Wheel<1000, TimerId>,
  seconds: Wheel<60, TimerId>,
  minutes: Wheel<60, TimerId>,
  hours: Wheel<24, TimerId>,
  days: Wheel<365, TimerId>,

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
  pub fn peek_nearest_timer(&self) -> Option<usize> {
    self
      .milliseconds
      .peak_nearest()
      .or(self.seconds.peak_nearest().map(|second| second * 1000))
      .or(self.minutes.peak_nearest().map(|minute| minute * 60 * 1000))
      .or(self.hours.peak_nearest().map(|hour| hour * 60 * 60 * 1000))
      .or(self.days.peak_nearest().map(|day| day * 24 * 60 * 60 * 1000))
  }

  pub fn start_elapsed(&self) -> usize {
    self.start_instant.elapsed().as_millis() as usize
  }

  pub fn insert(&mut self, timer: TimerId) {
    let delta = timer.timestamp().saturating_sub(self.start_elapsed());
    let (day_ticks, hour_ticks, minute_ticks, second_ticks, millisecond_ticks) =
      utils::breakdown_milliseconds(delta);

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
      // For Safety
      self.milliseconds.insert(1, timer);
    }
  }

  pub fn advance(
    &mut self,
    millis: usize,
  ) -> impl Iterator<Item = TimerId> + use<'_> {
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

    vec.into_iter().filter_map(|item| {
      if self.start_elapsed() >= item.timestamp() {
        return Some(item);
      } else {
        self.insert(item);
        return None;
      }
    })
  }
}

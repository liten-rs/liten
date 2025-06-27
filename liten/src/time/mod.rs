mod sleep;
use std::{
  cell::Cell,
  task::Waker,
  thread::JoinHandle,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub use sleep::*;

use crate::{loom::thread, sync::oneshot::Receiver};

#[derive(Debug)]
pub struct Timer {
  waker: Waker,

  /// duration from start instant to when timer is actuated.
  timestamp: usize,
}

impl Timer {
  fn new(waker: Waker, timestamp: usize) -> Self {
    Self { waker, timestamp }
  }
  fn trigger(self) {
    println!("triggered");
    self.waker.wake();
    // Actuate the waker
  }
}

pub struct TimerWheel<const T: usize, I> {
  slots: [Cell<Vec<I>>; T], // Fixed-size array of Vec<Timer>
  current_slot: Cell<usize>,
}

struct TimerTickResult<T> {
  slots: Vec<T>,
  resetted_counter: bool,
}

impl<const T: usize, I> TimerWheel<T, I> {
  fn new() -> Self {
    let slots =
      std::array::from_fn::<Cell<Vec<I>>, T, _>(|_| Cell::new(Vec::<I>::new()));
    TimerWheel { slots, current_slot: Cell::new(0) }
  }

  /// Overflow will be wrapped
  fn new_with_position(position: usize) -> Self {
    let slots =
      std::array::from_fn::<Cell<Vec<I>>, T, _>(|_| Cell::new(Vec::<I>::new()));
    TimerWheel { slots, current_slot: Cell::new(position % T) }
  }

  fn add_timer(&mut self, timer: I, ticks_forward: usize) {
    let slot = (self.current_slot.get() + ticks_forward) % T;
    let mut nice = self.slots[slot].take();
    nice.push(timer);

    self.slots[slot].set(nice);
  }

  fn tick(&self) -> TimerTickResult<I> {
    let current_slot = self.current_slot.get();
    let next_current_slot = (current_slot + 1) % T;
    self.current_slot.set(next_current_slot);
    let slots = self.slots[current_slot].take();

    TimerTickResult { slots, resetted_counter: next_current_slot == 0 }
  }
}

pub struct TimeDriver {
  milliseconds: TimerWheel<1000, Timer>,
  seconds: TimerWheel<60, Timer>,
  minutes: TimerWheel<60, Timer>,
  hours: TimerWheel<24, Timer>,
  days: TimerWheel<365, Timer>,

  started: Instant,

  current_dur_from_init: Cell<usize>,
}

impl TimeDriver {
  pub fn new() -> Self {
    let (days, hours, minutes, seconds, milliseconds) =
      utils::breakdown_milliseconds(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
          as usize,
      );
    Self {
      // milliseconds: TimerWheel::new(),
      // seconds: TimerWheel::new(),
      // minutes: TimerWheel::new(),
      // hours: TimerWheel::new(),
      // days: TimerWheel::new(),
      milliseconds: TimerWheel::new_with_position(milliseconds as usize),
      seconds: TimerWheel::new_with_position(seconds as usize),
      minutes: TimerWheel::new_with_position(minutes as usize),
      hours: TimerWheel::new_with_position(hours as usize),
      days: TimerWheel::new_with_position(days as usize),
      started: Instant::now(),
      current_dur_from_init: Cell::new(0),
    }
  }

  // fn get_dur_from_init(&self) -> usize {
  //   Instant::now().duration_since(self.started).as_millis() as usize
  // }

  pub fn add_waker(&mut self, waker: Waker, absolute_duration: Duration) {
    let init_dur = self.current_dur_from_init.get();
    let absolute_duration = absolute_duration.as_millis() as usize;

    assert!(init_dur < absolute_duration, "{init_dur} !< {absolute_duration}");

    // // Duration is from now so needs to be synced to time driver start.
    // let timestamp = self.get_dur_from_init() + dbg!(duration);
    let (days, hours, minutes, seconds, milliseconds) =
      utils::breakdown_milliseconds(absolute_duration - init_dur);
    //
    let timer = Timer::new(waker, absolute_duration);

    if days != 0 {
      self.days.add_timer(timer, days - 1);
    } else if hours != 0 {
      self.hours.add_timer(timer, hours - 1);
    } else if minutes != 0 {
      self.minutes.add_timer(timer, minutes - 1);
    } else if seconds != 0 {
      self.seconds.add_timer(timer, seconds - 1);
    } else if milliseconds != 0 {
      self.milliseconds.add_timer(timer, milliseconds - 1);
    } else {
      panic!("all is 0");
    }
  }

  pub fn tick(&mut self) {
    let now = self.current_dur_from_init.get();
    self
      .current_dur_from_init
      .set(Instant::now().duration_since(self.started).as_millis() as usize);

    let TimerTickResult { slots, resetted_counter } = self.milliseconds.tick();
    self.handle_slots(slots, now);

    if resetted_counter {
      let TimerTickResult { slots, resetted_counter } = self.seconds.tick();
      self.handle_slots(slots, now);

      if resetted_counter {
        let TimerTickResult { slots, resetted_counter } = self.minutes.tick();
        self.handle_slots(slots, now);

        if resetted_counter {
          let TimerTickResult { slots, resetted_counter } = self.hours.tick();
          self.handle_slots(slots, now);

          if resetted_counter {
            let TimerTickResult { slots, resetted_counter: _ } =
              self.days.tick();
            self.handle_slots(slots, now);
          }
        }
      }
    }
  }

  fn handle_slots(&mut self, slots: Vec<Timer>, dur_from_init: usize) {
    for slot in slots {
      if dur_from_init >= slot.timestamp {
        slot.trigger();
      } else {
        self
          .add_waker(slot.waker, Duration::from_millis(slot.timestamp as u64));
      }
    }
  }

  pub fn launch(mut self, oneshot_receiver: Receiver<()>) -> JoinHandle<()> {
    thread::spawn(move || loop {
      let instant = Instant::now();

      if oneshot_receiver.try_recv().expect("Shouldn't error").is_some() {
        break;
      }

      self.tick();

      loop {
        std::thread::yield_now();

        let instant2 = Instant::now();
        if instant2 - instant >= Duration::from_millis(1) {
          break;
        }
      }
    })
  }
}

mod utils {
  pub(super) fn breakdown_milliseconds(
    total_ms: usize,
  ) -> (usize, usize, usize, usize, usize) {
    let milliseconds = total_ms % 1000;
    let seconds = (total_ms / 1000) % 60;
    let minutes = (total_ms / (1000 * 60)) % 60;
    let hours = (total_ms / (1000 * 60 * 60)) % 24;
    let days = total_ms / (1000 * 60 * 60 * 24);

    (days, hours, minutes, seconds, milliseconds)
  }
}

#[test]
fn timer_playground() {
  let mut mng = TimeDriver::new();

  mng.add_waker(futures_task::noop_waker(), Duration::from_millis(10600));

  let target_interval = Duration::from_millis(1);
  let start_time = Instant::now();
  let mut next_tick = target_interval;

  loop {
    // Your algorithm's logic here
    mng.tick();

    // Calculate the time until the next tick
    let now = Instant::now();
    let elapsed = now - start_time;

    if elapsed < next_tick {
      // Sleep for most of the remaining time
      let sleep_duration = next_tick - elapsed;
      if sleep_duration > Duration::from_micros(100) {
        std::thread::sleep(sleep_duration - Duration::from_micros(100));
      }

      // Use a short spin-wait loop to fine-tune the timing
      let spin_start = Instant::now();
      while spin_start.elapsed() < Duration::from_micros(200) {}
    } else {
      // If we've fallen behind, adjust next_tick to catch up
      next_tick = elapsed + target_interval;
    }

    // Schedule the next tick
    next_tick += target_interval;
  }

  // loop {
  //   let instant = Instant::now();
  //
  //   loop {
  //     std::thread::yield_now();
  //
  //     let instant2 = Instant::now();
  //     if instant2 - instant >= Duration::from_millis(1) {
  //       break;
  //     }
  //   }
  // }
}

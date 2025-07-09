mod clock;
mod sleep;
mod utils;
mod wheel;

use std::{
  collections::HashMap,
  sync::{atomic::AtomicUsize, OnceLock},
  task::{Context, Poll, Waker},
  thread::{self, JoinHandle},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::loom::sync::{Arc, Mutex};

use clock::{Clock, TimerId};
use parking::{Parker, Unparker};
pub use sleep::*;

#[derive(Clone)]
pub struct TimeDriver(Arc<Mutex<TimeDriverInner>>);

pub struct TimeDriverInner {
  clock: Clock,
  last_advance: Instant,

  shutdown_signal: bool,
  waker_store: HashMap<TimerId, Waker>,
  background_handle: Option<JoinHandle<()>>,
  unparker: Unparker,
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
impl TimeDriver {
  fn new() -> TimeDriver {
    let (days, hours, minutes, seconds, milliseconds) =
      utils::breakdown_milliseconds(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
          as usize,
      );
    let clock =
      Clock::new_with_positions(days, hours, minutes, seconds, milliseconds);

    let parker = Parker::new();

    let time_driver_inner = TimeDriverInner {
      clock: clock,
      last_advance: Instant::now(),
      shutdown_signal: false,
      background_handle: None,
      waker_store: HashMap::new(),
      unparker: parker.unparker(),
    };

    let driver = TimeDriver(Arc::new(Mutex::new(time_driver_inner)));

    let driver_clone = driver.clone();
    let mut driver_lock = driver_clone.0.lock().unwrap();

    driver_lock.background_handle = Some(thread::spawn({
      let driver = driver.clone();
      move || driver.background_thread(parker)
    }));
    drop(driver_lock);

    driver
  }

  pub(in crate::time) fn start_elapsed(&self) -> usize {
    let _lock = self.0.lock().unwrap();
    let value = _lock.clock.start_elapsed();
    drop(_lock);
    value
  }

  pub fn shutdown() {
    static DONE_BEFORE: OnceLock<()> = OnceLock::new();

    if DONE_BEFORE.get().is_some() {
      panic!("shutdown after shutdown");
    }
    let this = Self::get();

    let mut _lock = this.0.lock().unwrap();

    _lock.shutdown_signal = true;
    _lock.unparker.unpark();

    let handle = _lock.background_handle.take().unwrap();
    drop(_lock);
    let _ = handle.join();
    let _ = DONE_BEFORE.set(());
  }

  pub fn get_timer_ticket() -> usize {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
  }

  fn background_thread(&self, parker: Parker) {
    loop {
      let lock = self.0.lock().unwrap();

      let nearest = lock.clock.peek_nearest_timer();
      drop(lock);

      match nearest {
        Some(delta_time_to_next_thing) => {
          // There is one
          let instant = Instant::now()
            + Duration::from_millis(delta_time_to_next_thing as u64);
          parker.park_deadline(instant);
        }
        None => {
          // No timers currently waiting...
          parker.park();
        }
      }
      if self.0.lock().unwrap().shutdown_signal {
        break;
      }
      self.jump();
    }
  }
}

impl TimeDriver {
  fn get() -> &'static TimeDriver {
    static TIME_DRIVER: OnceLock<TimeDriver> = OnceLock::new();
    TIME_DRIVER.get_or_init(TimeDriver::new)
  }

  pub fn insert(&self, duration: usize) -> TimerId {
    let timestamp = self.start_elapsed() + duration;
    let timer = TimerId::new(timestamp);

    let mut _lock = self.0.lock().unwrap();
    _lock.clock.insert(timer);
    _lock.unparker.unpark();

    timer
  }

  pub fn poll(&self, cx: &mut Context, timer_id: TimerId) -> Poll<()> {
    if timer_id.has_happened(self.start_elapsed()) {
      self.0.lock().unwrap().waker_store.remove(&timer_id);
      return Poll::Ready(());
    }

    self.0.lock().unwrap().waker_store.insert(timer_id, cx.waker().clone());

    Poll::Pending
  }

  fn jump(&self) {
    let mut _lock = self.0.lock().unwrap();

    let millis = _lock.last_advance.elapsed().as_millis() as usize;

    let timers: Vec<TimerId> = _lock.clock.advance(millis).collect();
    _lock.last_advance = Instant::now();

    for timer in timers {
      if let Some(testing) = _lock.waker_store.remove(&timer) {
        testing.wake();
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

  fn dummy_waker() -> Waker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
      dummy_raw_waker()
    }
    static VTABLE: RawWakerVTable =
      RawWakerVTable::new(clone, no_op, no_op, no_op);
    fn dummy_raw_waker() -> RawWaker {
      RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe { Waker::from_raw(dummy_raw_waker()) }
  }

  #[crate::internal_test]
  fn timer_insert_and_poll_integration() {
    let driver = TimeDriver::get();
    let timer_id = driver.insert(1000);
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    let poll = driver.poll(&mut cx, timer_id);
    assert!(matches!(poll, Poll::Pending));
    TimeDriver::shutdown();
  }

  #[crate::internal_test]
  #[should_panic]
  fn shutdown_twice_panics_integration() {
    TimeDriver::shutdown();
    TimeDriver::shutdown();
  }
}

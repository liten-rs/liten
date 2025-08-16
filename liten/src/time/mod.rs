mod clock;
mod sleep;
mod utils;
mod wheel;

use std::{
  collections::HashMap,
  sync::{atomic::AtomicUsize, OnceLock},
  task::{Context, Poll, Waker},
  thread::{self, JoinHandle, Thread},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::loom::sync::{Arc, Mutex};

use clock::{Clock, TimerId};
pub use sleep::*;

#[derive(Clone)]
pub struct TimeDriver {
  field1: Arc<Mutex<TimeDriverInner>>,
  // thread: OnceLock<Thread>,
}

pub struct TimeDriverInner {
  clock: Clock,
  last_advance: Instant,

  shutdown_signal: bool,
  waker_store: HashMap<TimerId, Waker>,
  background_handle: Option<JoinHandle<()>>,
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
impl TimeDriver {
  fn get() -> &'static TimeDriver {
    static TIME_DRIVER: OnceLock<TimeDriver> = OnceLock::new();
    let driver = TIME_DRIVER.get_or_init(|| TimeDriver::new());
    driver
  }
  fn new() -> TimeDriver {
    let (days, hours, minutes, seconds, milliseconds) =
      utils::breakdown_milliseconds(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
          as usize,
      );
    let clock =
      Clock::new_with_positions(days, hours, minutes, seconds, milliseconds);

    // let parker = Parker::new();

    let time_driver_inner = TimeDriverInner {
      clock,
      last_advance: Instant::now(),
      shutdown_signal: false,
      background_handle: None,
      waker_store: HashMap::new(),
    };

    let driver = TimeDriver {
      field1: Arc::new(Mutex::new(time_driver_inner)),
      // thread: OnceLock::new(),
    };

    let driver_clone = driver.clone();
    let mut driver_lock = driver_clone.field1.lock().unwrap();

    driver_lock.background_handle = Some(thread::spawn({
      let driver = driver.clone();
      move || driver.background_thread()
    }));
    drop(driver_lock);

    driver
  }

  pub(in crate::time) fn start_elapsed(&self) -> usize {
    let _lock = self.field1.lock().unwrap();
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

    let mut _lock = this.field1.lock().unwrap();

    _lock.shutdown_signal = true;

    let handle = _lock.background_handle.take().unwrap();
    handle.thread().unpark();
    drop(_lock);
    let _ = handle.join();
    let _ = DONE_BEFORE.set(());
  }

  pub fn get_timer_ticket() -> usize {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
  }

  #[cfg(not(loom))]
  fn background_thread(&self) {
    const FUDGE_MS: usize = 10;
    let driver = TimeDriver::get();

    loop {
      self.jump();

      let lock = self.field1.lock().unwrap();
      let nearest = lock.clock.peek_nearest_timer();
      let shutdown = lock.shutdown_signal;
      drop(lock);

      if shutdown {
        break;
      }

      match nearest {
        Some(delta_time_to_next_thing) => {
          let sleep_ms = if delta_time_to_next_thing > FUDGE_MS {
            delta_time_to_next_thing - FUDGE_MS
          } else {
            delta_time_to_next_thing
          };
          let instant = Instant::now() + Duration::from_millis(sleep_ms as u64);

          std::thread::park_timeout(
            instant.saturating_duration_since(Instant::now()),
          );

          // Busy-wait for the last few ms
          let target = Instant::now() + Duration::from_millis(FUDGE_MS as u64);
          while Instant::now() < target {
            std::hint::spin_loop();
          }
        }
        None => {
          // No timers currently waiting...
          std::thread::park();
        }
      }
    }
  }
}

impl TimeDriver {
  pub fn insert(&self, duration: usize) -> TimerId {
    let timestamp = self.start_elapsed() + duration;
    let timer = TimerId::new(timestamp);

    let mut _lock = self.field1.lock().unwrap();
    _lock.clock.insert(timer);
    if let Some(handle) = _lock.background_handle.as_ref() {
      handle.thread().unpark();
    } else {
      unreachable!()
    };

    timer
  }

  pub fn poll(&self, cx: &mut Context, timer_id: TimerId) -> Poll<()> {
    // Drive time forward on poll to ensure progress even if background thread isn't running
    self.jump();

    if timer_id.has_happened(self.start_elapsed()) {
      self.field1.lock().unwrap().waker_store.remove(&timer_id);
      return Poll::Ready(());
    }

    self
      .field1
      .lock()
      .unwrap()
      .waker_store
      .insert(timer_id, cx.waker().clone());

    Poll::Pending
  }

  fn jump(&self) {
    let mut _lock = self.field1.lock().unwrap();

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
    let timer_id = driver.insert(10000);
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

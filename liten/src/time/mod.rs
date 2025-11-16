mod clock;
mod sleep;
mod utils;
mod wheel;

use dashmap::{DashMap, Entry};
use oneshot::Receiver;
pub use sleep::Sleep;

use crate::{
  loom::sync::{
    atomic::AtomicUsize,
    mpsc::{self, TryRecvError},
    Arc,
  },
  time::clock::TimerEntry,
};

use std::{
  collections::HashMap,
  future::Future,
  sync::{atomic::Ordering, OnceLock},
  task::Waker,
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

use clock::{Clock, TimerId};
use parking_lot::Mutex;

const SPIN_THRESHOLD_MS: u64 = 10;

pub struct TimeHandle {
  sender: mpsc::Sender<DriverMessage>,
  background_handle: Option<JoinHandle<()>>,
  state: Arc<TimeState>,
}

#[derive(Default)]
struct TimeState {
  state: DashMap<TimerId, TimerVariant>,
  next_id: AtomicUsize,
}

pub enum TimerVariant {
  Waker(Waker),
  Fn(Box<dyn FnOnce() + Send + Sync>),
}

impl std::fmt::Debug for TimerVariant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Waker(_) => f.debug_tuple("Waker").finish(),
      Self::Fn(_) => f.debug_tuple("Fn").finish(),
    }
  }
}

impl TimerVariant {
  fn is_variant_eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Waker(_), Self::Waker(_)) => true,
      (Self::Fn(_), Self::Fn(_)) => true,
      _ => false,
    }
  }
}

impl TimerVariant {
  fn call(self) {
    match self {
      Self::Waker(waker) => waker.wake(),
      Self::Fn(_fn) => _fn(),
    };
  }
}

#[derive(Debug)]
pub struct DriverMessage {
  confirmer: oneshot::Sender<usize>,
  payload: MessagePayload,
}

#[derive(Debug)]
pub enum MessagePayload {
  Shutdown,
  AddTimer { variant: TimerVariant, duration: usize },
  UpdateTimer { variant: TimerVariant, timer_id: TimerId },
}

#[derive(Debug)]
pub struct MessageNotReceived;

impl TimeHandle {
  fn get<F, R>(f: F) -> R
  where
    F: FnOnce(&mut TimeHandle) -> R,
  {
    static TIME_DRIVER: Mutex<Option<TimeHandle>> = Mutex::new(None);
    let mut _lock = TIME_DRIVER.lock();

    let res = match _lock.as_mut() {
      Some(handle) => f(handle),
      None => {
        *_lock = Some(TimeHandle::new());
        f(_lock.as_mut().unwrap())
      }
    };

    drop(_lock);

    res
  }

  fn take_handle(&mut self) -> JoinHandle<()> {
    self.background_handle.take().expect("lio error: Driver background worker handle doesn't exist in handle or has already been taken")
  }

  fn new() -> TimeHandle {
    let (sender, receiver) = mpsc::channel();
    let state = Arc::new(TimeState::default());

    let _state = state.clone();
    let handle = thread::spawn(move || Self::background(receiver, _state));

    let time_driver =
      TimeHandle { background_handle: Some(handle), sender, state };

    time_driver
  }

  fn send_message(&self, payload: MessagePayload) -> Receiver<usize> {
    let (sender, receiver) = oneshot::channel();
    if let Err(err) =
      self.sender.send(DriverMessage { payload, confirmer: sender })
    {
      panic!("lio error: Driver background worker cannot receiver messagees even when initiated: {err:#?}");
    };

    let handle = self
      .background_handle
      .as_ref()
      .expect("driver not launched before sending message");

    handle.thread().unpark();

    receiver

    // match receiver.recv_timeout(Duration::from_millis(1_000)) {
    //     Ok(value) => value,
    //     Err(_) =>  panic!("lio error: Driver background worker cannot receiver messagees even when initiated."),
    // }
  }

  fn background(
    receiver: mpsc::Receiver<DriverMessage>,
    state: Arc<TimeState>,
  ) {
    let mut clock = Clock::new();
    let mut last_advance = Instant::now();

    'outer: loop {
      println!("start iter");
      loop {
        println!("message iter");
        let message = match dbg!(receiver.try_recv()) {
          Ok(value) => value,
          Err(err) => match err {
            TryRecvError::Empty => break,
            TryRecvError::Disconnected => break 'outer,
          },
        };
        println!("received message {:#?}", message.payload);

        match message.payload {
          MessagePayload::Shutdown => {
            println!("shutting down");
            let _ = message.confirmer.send(0);
            break 'outer;
          }
          MessagePayload::AddTimer { variant, duration } => {
            let next_id = state.next_id.fetch_add(1, Ordering::AcqRel);
            let timer_id = TimerId::new(next_id);
            clock.insert(timer_id, duration);
            if let Some(old_insert) = state.state.insert(timer_id, variant) {
              old_insert;
            };
            let _ = message.confirmer.send(next_id);
          }
          MessagePayload::UpdateTimer { timer_id, variant } => {
            println!("updating timer");
            match state.state.entry(timer_id) {
              Entry::Vacant(_) => {
                let _ = message.confirmer.send(1);
              }
              Entry::Occupied(entry) => {
                if variant.is_variant_eq(entry.get()) {
                  let _ = entry.replace_entry(variant);
                  let _ = message.confirmer.send(0);
                } else {
                  let _ = message.confirmer.send(1);
                };
              }
            }
          }
        }
      }

      let to_advance = last_advance.elapsed();
      let timers = clock.advance(to_advance.as_millis() as usize);
      last_advance += to_advance;

      println!("calling");
      for timer_id in timers {
        // If doesn't exist, means it's cancelled.
        if let Some((_, entry)) = state.state.remove(&timer_id) {
          entry.call();
        }
      }
      println!("calling end");

      loop {
        println!("message iter");
        let message = match dbg!(receiver.try_recv()) {
          Ok(value) => value,
          Err(err) => match err {
            TryRecvError::Empty => break,
            TryRecvError::Disconnected => break 'outer,
          },
        };
        println!("received message {:#?}", message.payload);

        match message.payload {
          MessagePayload::Shutdown => {
            println!("shutting down");
            let _ = message.confirmer.send(0);
            break 'outer;
          }
          MessagePayload::AddTimer { variant, duration } => {
            let next_id = state.next_id.fetch_add(1, Ordering::AcqRel);
            let timer_id = TimerId::new(next_id);
            clock.insert(timer_id, duration);
            if let Some(old_insert) = state.state.insert(timer_id, variant) {
              old_insert;
            };
            let _ = message.confirmer.send(next_id);
          }
          MessagePayload::UpdateTimer { timer_id, variant } => {
            println!("updating timer");
            match state.state.entry(timer_id) {
              Entry::Vacant(_) => {
                let _ = message.confirmer.send(1);
              }
              Entry::Occupied(entry) => {
                if variant.is_variant_eq(entry.get()) {
                  let _ = entry.replace_entry(variant);
                  let _ = message.confirmer.send(0);
                } else {
                  let _ = message.confirmer.send(1);
                };
              }
            }
          }
        }
      }

      match dbg!(clock.peek()) {
        Some(timeout_ms) => {
          if timeout_ms <= SPIN_THRESHOLD_MS as usize {
            // For very short timeouts, just spin
            let deadline =
              Instant::now() + Duration::from_millis(timeout_ms as u64);
            while Instant::now() < deadline {
              std::hint::spin_loop();
            }
          } else {
            // For longer timeouts: park early, then spin the final portion
            let park_duration =
              timeout_ms.saturating_sub(SPIN_THRESHOLD_MS as usize);
            thread::park_timeout(Duration::from_millis(park_duration as u64));

            // Spin-wait for the remaining time to hit the deadline accurately
            let deadline =
              Instant::now() + Duration::from_millis(SPIN_THRESHOLD_MS);
            while Instant::now() < deadline {
              std::hint::spin_loop();
            }
          }
        }
        None => thread::park(),
      }
      println!("end iter");
    }
  }
}

impl TimeHandle {
  pub fn shutdown() {
    let receiver = Self::get(move |h| {
      h.send_message(MessagePayload::Shutdown)
      // let
    });
    let code = receiver.recv().expect("liten time error;");
    assert!(code == 0, "Shutdown response code not valid.");
    Self::get(|h| {
      h.take_handle().join();
    })
  }

  /// Can be called without awaiting. when awaiting, what you really do is confirm the message has
  /// gone through to the driver.
  pub fn add(variant: TimerVariant, duration: usize) -> TimerId {
    let receiver = Self::get(move |h| {
      h.send_message(MessagePayload::AddTimer { variant, duration })
    });

    TimerId::new(receiver.recv().expect("liten time error"))
  }

  pub fn add_waker(waker: Waker, duration: usize) -> TimerId {
    Self::add(TimerVariant::Waker(waker), duration)
  }

  pub fn add_fn<F>(_fn: F, duration: usize) -> TimerId
  where
    F: FnOnce() + Send + Sync + 'static,
  {
    Self::add(TimerVariant::Fn(Box::new(_fn)), duration)
  }

  /// Panics if variant is not waker and if entry doesn't exist.
  pub fn update_timer_waker(timer_id: TimerId, waker: Waker) {
    let receiver = Self::get(move |h| {
      h.send_message(MessagePayload::UpdateTimer {
        variant: TimerVariant::Waker(waker),
        timer_id,
      })
    });
    let code = receiver.recv().expect("liten time error");

    assert!(code == 0);
  }

  pub fn entry_exists(timer_id: &TimerId) -> bool {
    Self::get(move |h| h.state.state.contains_key(timer_id))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{
    sync::{
      atomic::{AtomicBool, Ordering},
      Arc,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
  };

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
    let thing = Arc::new(AtomicBool::new(false));
    let _thing = thing.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let driver = TimeHandle::add_fn(
      move || {
        sender.send(0);
      },
      200,
    );

    receiver.recv_timeout(Duration::from_millis(205)).unwrap();

    TimeHandle::shutdown();
  }

  #[crate::internal_test]
  #[should_panic]
  fn shutdown_twice_panics_integration() {
    TimeHandle::shutdown();
    TimeHandle::shutdown();
  }
}

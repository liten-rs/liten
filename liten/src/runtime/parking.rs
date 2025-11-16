use std::{
  sync::OnceLock,
  thread::{self, Thread},
};

static WAITER: OnceLock<Thread> = OnceLock::new();

pub fn set_thread() {
  WAITER.set(thread::current()).expect("Failed to set thread");
}

pub fn unpark() {
  let waiter = WAITER.get().expect("'liten' not initialized");
  waiter.unpark();
}

pub fn park() {
  thread::park();
}

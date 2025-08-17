use std::{
  sync::OnceLock,
  thread::{self, Thread},
};

static WAITER: OnceLock<Thread> = OnceLock::new();

pub fn set_thread(thread: Thread) {
  if let Err(_) = WAITER.set(thread) {
    panic!("oh no!");
  }
}

pub fn set_main_thread() -> Thread {
  let thread = thread::current();
  set_thread(thread.clone());
  thread
}

pub fn unpark() {
  let waiter = WAITER.get().expect("'liten' not initialized");
  waiter.unpark();
}

pub fn park() {
  thread::park();
}

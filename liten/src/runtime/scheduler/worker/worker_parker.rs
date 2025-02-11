use std::sync::{Condvar, Mutex};

pub struct WorkerParker {
  lock: Mutex<()>,
  condvar: Condvar,
}

impl WorkerParker {
  pub fn new() -> Self {
    Self { lock: Mutex::new(()), condvar: Condvar::new() }
  }

  pub fn park(&self) {
    let _guard = self.lock.lock().unwrap();
    let _drop = self.condvar.wait(_guard).unwrap();
    println!("done");
  }

  pub fn unpark(&self) {
    self.condvar.notify_one();
  }
}

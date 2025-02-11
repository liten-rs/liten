pub mod worker;

use std::sync::Arc;

use super::super::io_loop as io;

use crate::task::ArcTask;

#[derive(Debug)]
pub struct Scheduler;

pub struct Handle {
  pub io: io::Handle,
  pub shared: Arc<worker::Shared>,
}

impl Handle {
  pub fn new(io: io::Handle, state: Arc<worker::Shared>) -> Handle {
    Handle { io, shared: state }
  }
}

pub struct Driver {
  pub io: io::Driver,
}

impl Handle {
  pub fn state(&self) -> &worker::Shared {
    &self.shared
  }
}

impl Handle {
  pub fn io(&self) -> &io::Handle {
    &self.io
  }
}

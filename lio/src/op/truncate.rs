use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::{opcode, squeue, types::Fd};

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Truncate {
  fd: RawFd,
  size: u64,
}

impl Truncate {
  pub(crate) fn new(fd: RawFd, size: u64) -> Self {
    Self { fd, size }
  }
}

impl Operation for Truncate {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 55;

  #[cfg(linux)]
  fn create_entry(&mut self) -> squeue::Entry {
    opcode::Ftruncate::new(Fd(self.fd), self.size).build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(ftruncate(self.fd, self.size as i64))
  }
}

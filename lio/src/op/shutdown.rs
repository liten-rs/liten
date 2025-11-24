use std::{io, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Shutdown {
  fd: RawFd,
  how: i32,
}

impl Shutdown {
  pub fn new(fd: RawFd, how: i32) -> Self {
    Self { fd, how }
  }
}

impl Operation for Shutdown {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 34;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Shutdown::new(Fd(self.fd), self.how).build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = Some(EventType::Write);

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  fn run_blocking(&self) -> io::Result<i32> {
    syscall!(shutdown(self.fd, self.how)).map(|t| t as i32)
  }
}

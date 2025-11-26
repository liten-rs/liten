use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Listen {
  fd: RawFd,
  backlog: i32,
}

impl Listen {
  pub(crate) fn new(fd: RawFd, backlog: i32) -> Self {
    Self { fd, backlog }
  }
}

impl Operation for Listen {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 57;

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Listen::new(Fd(self.fd), self.backlog).build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(listen(self.fd, self.backlog))
  }
}

use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::{opcode, types::Fd};

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Close {
  fd: RawFd,
}

impl Close {
  pub(crate) fn new(fd: RawFd) -> Self {
    Self { fd }
  }
}

impl Operation for Close {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 19;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    opcode::Close::new(Fd(self.fd)).build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(close(self.fd))
  }
}

use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Fsync {
  fd: RawFd,
}

impl Fsync {
  pub(crate) fn new(fd: RawFd) -> Self {
    Self { fd }
  }
}

impl Operation for Fsync {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 3;

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Fsync::new(Fd(self.fd)).build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(fsync(self.fd))
  }
}

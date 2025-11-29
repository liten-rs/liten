#[cfg(not(linux))]
use crate::op::EventType;
use std::{io, os::fd::RawFd};

use io_uring::types::Fd;

use super::Operation;

// TODO: not sure detach safe.
pub struct Tee {
  fd_in: RawFd,
  fd_out: RawFd,
  size: u32,
}

impl Tee {
  pub(crate) fn new(fd_in: RawFd, fd_out: RawFd, size: u32) -> Self {
    Self { fd_in, fd_out, size }
  }
}

impl Operation for Tee {
  #[cfg(linux)]
  const OPCODE: u8 = 33;

  type Result = io::Result<Self::Output>;
  type Output = i32;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::Tee::new(Fd(self.fd_in), Fd(self.fd_out), self.size)
      .build()
  }

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(tee(self.fd_in, self.fd_out, self.size as usize, 0))
      .map(|s| s as i32)
  }

  fn result(&mut self, _ret: std::io::Result<i32>) -> Self::Result {
    _ret
  }
}

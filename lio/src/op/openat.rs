use std::{ffi::CString, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct OpenAt {
  fd: RawFd,
  pathname: CString,
  flags: i32,
}

impl OpenAt {
  pub fn new(fd: RawFd, pathname: CString, flags: i32) -> Self {
    Self { fd, pathname, flags }
  }
}

impl Operation for OpenAt {
  #[cfg(unix)]
  type Output = std::os::fd::RawFd;

  #[cfg(unix)]
  type Result = std::io::Result<Self::Output>;
  /// File descriptor returned from the operation.
  fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
    fd
  }

  #[cfg(linux)]
  const OPCODE: u8 = 18;

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::OpenAt::new(Fd(self.fd), self.pathname.as_ptr())
      .flags(self.flags)
      .build()
  }
  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(openat(self.fd, self.pathname.as_ptr(), self.flags))
  }
}

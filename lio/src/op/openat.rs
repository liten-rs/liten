use std::{ffi::CString, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

use super::Operation;

pub struct OpenAt {
  fd: RawFd,
  pathname: CString,
  flags: i32,
}

impl OpenAt {
  pub(crate) fn new(fd: RawFd, pathname: CString, flags: i32) -> Self {
    Self { fd, pathname, flags }
  }
}

impl Operation for OpenAt {
  impl_result!(fd);

  #[cfg(linux)]
  const OPCODE: u8 = 18;

  impl_no_readyness!();

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::OpenAt::new(Fd(self.fd), self.pathname.as_ptr())
      .flags(self.flags)
      .build()
  }
  #[cfg(not(linux))]
  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(openat(self.fd, self.pathname.as_ptr(), self.flags))
  }
}

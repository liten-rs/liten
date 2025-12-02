use std::{io, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

use crate::op::DetachSafe;

use super::Operation;

pub struct Shutdown {
  fd: RawFd,
  how: i32,
}

impl Shutdown {
  pub(crate) fn new(fd: RawFd, how: i32) -> Self {
    Self { fd, how }
  }
}

unsafe impl DetachSafe for Shutdown {}

impl Operation for Shutdown {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 34;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::Shutdown::new(Fd(self.fd), self.how).build()
  }

  // NOTE: Not sure here, kqueue can prob be used with flags.
  impl_no_readyness!();

  #[cfg(not(linux))]
  fn run_blocking(&self) -> io::Result<i32> {
    syscall!(shutdown(self.fd, self.how))
  }
}

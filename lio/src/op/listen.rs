use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::types::Fd;

use crate::op::DetachSafe;

use super::Operation;

pub struct Listen {
  fd: RawFd,
  backlog: i32,
}

unsafe impl DetachSafe for Listen {}

impl Listen {
  pub(crate) fn new(fd: RawFd, backlog: i32) -> Self {
    Self { fd, backlog }
  }
}

impl Operation for Listen {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 57;

  impl_no_readyness!();

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::Listen::new(Fd(self.fd), self.backlog).build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(listen(self.fd, self.backlog))
  }
}

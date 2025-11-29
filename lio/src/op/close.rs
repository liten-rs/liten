use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::{opcode, types::Fd};

use crate::op::DetachSafe;

use super::Operation;

pub struct Close {
  fd: RawFd,
}

unsafe impl DetachSafe for Close {}

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
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    opcode::Close::new(Fd(self.fd)).build()
  }

  impl_no_readyness!();

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(close(self.fd))
  }
}

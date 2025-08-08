use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Close {
  fd: RawFd,
}

impl Close {
  pub fn new(fd: RawFd) -> Self {
    Self { fd }
  }
}

impl Operation for Close {
  impl_result!(());
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Close::new(Fd(self.fd)).build()
  }
}

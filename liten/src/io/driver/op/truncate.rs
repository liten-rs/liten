use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Truncate {
  fd: RawFd,
  size: u64,
}

impl Truncate {
  pub fn new(fd: RawFd, size: u64) -> Self {
    Self { fd, size }
  }
}

impl Operation for Truncate {
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Ftruncate::new(Fd(self.fd), self.size).build()
  }

  impl_result!(());
}

use std::os::fd::RawFd;

os_linux! {
  use io_uring::types::Fd;
}

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
  impl_result!(());

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Ftruncate::CODE;
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Ftruncate::new(Fd(self.fd), self.size).build()
    }
  }
  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(ftruncate(self.fd, self.size as i64))
  }
}

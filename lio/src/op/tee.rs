use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Tee {
  fd_in: RawFd,
  fd_out: RawFd,
  size: u32,
}

impl Tee {
  pub fn new(fd_in: RawFd, fd_out: RawFd, size: u32) -> Self {
    Self { fd_in, fd_out, size }
  }
}

impl Operation for Tee {
  impl_result!(());

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Tee::CODE;
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Tee::new(Fd(self.fd_in), Fd(self.fd_out), self.size)
        .build()
    }
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(tee(self.fd_in, self.fd_out, self.size as usize, 0))
      .map(|s| s as i32)
  }
}

use std::os::fd::RawFd;

os_linux! {
  use io_uring::types::Fd;
}

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
  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Close::CODE;
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Close::new(Fd(self.fd)).build()
    }
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(close(self.fd))
  }
}

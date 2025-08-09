use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Listen {
  fd: RawFd,
  backlog: i32,
}

impl Listen {
  pub fn new(fd: RawFd, backlog: i32) -> Self {
    assert!(backlog > 0);
    Self { fd, backlog }
  }
}

impl Operation for Listen {
  impl_result!(());

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Listen::CODE;
    fn run_blocking(&self) -> std::io::Result<i32> {
      syscall!(listen(self.fd, self.backlog))
    }
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Listen::new(Fd(self.fd), self.backlog).build()
    }
  }
}

use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Listen {
  fd: RawFd,
  backlog: i32,
}

impl Listen {
  pub fn new(fd: RawFd, backlog: i32) -> Self {
    assert!(backlog < 0);
    Self { fd, backlog }
  }
}

impl Operation for Listen {
  type Output = ();
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Listen::new(Fd(self.fd), self.backlog).build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

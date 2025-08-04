use std::os::fd::RawFd;

use io_uring::types::Fd;
use socket2::SockAddr;

use super::Operation;

pub struct Connect {
  fd: RawFd,
  addr: SockAddr,
}

impl Connect {
  pub fn new(fd: RawFd, addr: SockAddr) -> Self {
    Self { fd, addr }
  }
}

impl Operation for Connect {
  type Output = (); // The file descriptor comes from the other end.
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Connect::new(
      Fd(self.fd),
      self.addr.as_ptr() as *const libc::sockaddr,
      self.addr.len(),
    )
    .build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Bind {
  fd: RawFd,
  addr: socket2::SockAddr,
}
impl Bind {
  pub fn new(fd: RawFd, addr: socket2::SockAddr) -> Self {
    Self { fd, addr }
  }
}

impl Operation for Bind {
  type Output = (); // The file descriptor comes from the other end.
  fn create_entry(&self) -> io_uring::squeue::Entry {
    let storage = self.addr.as_ptr();
    io_uring::opcode::Bind::new(
      Fd(self.fd),
      storage as *const libc::sockaddr,
      self.addr.len(),
    )
    .build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

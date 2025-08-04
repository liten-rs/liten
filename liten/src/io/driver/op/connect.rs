use std::{net::SocketAddr, os::fd::RawFd};

use io_uring::types::Fd;

use crate::io::utils;

use super::Operation;

pub struct Connect {
  fd: RawFd,
  addr: SocketAddr,
}

impl Connect {
  pub fn new(fd: RawFd, addr: SocketAddr) -> Self {
    Self { fd, addr }
  }
}

impl Operation for Connect {
  type Output = (); // The file descriptor comes from the other end.
  fn create_entry(&self) -> io_uring::squeue::Entry {
    let (addr, len) = utils::net::socket_addr_to_c(&self.addr);
    io_uring::opcode::Connect::new(Fd(self.fd), addr.as_ptr() as *const _, len)
      .build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

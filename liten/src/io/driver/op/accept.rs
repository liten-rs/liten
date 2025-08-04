use std::{mem::MaybeUninit, net::SocketAddr, os::fd::RawFd};

use io_uring::types::Fd;

use super::Operation;

pub struct Accept {
  fd: RawFd,
  addr: *mut MaybeUninit<libc::sockaddr_storage>,
  len: *mut libc::socklen_t,
}

impl Accept {
  pub fn new(
    fd: RawFd,
    addr: *mut MaybeUninit<libc::sockaddr_storage>,
    len: *mut libc::socklen_t,
  ) -> Self {
    Self { fd, addr, len }
  }
}

impl Operation for Accept {
  type Output = (); // The file descriptor comes from the other end.
  fn create_entry(&self) -> io_uring::squeue::Entry {
    // let (addr, len) = net_utils::socket_addr_to_c(&self.addr);
    return io_uring::opcode::Accept::new(
      Fd(self.fd),
      self.addr as *mut libc::sockaddr,
      self.len,
    )
    .build();
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

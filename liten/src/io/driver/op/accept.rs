use std::{mem::MaybeUninit, os::fd::RawFd};

use io_uring::types::Fd;
use socket2::SockAddrStorage;

use super::Operation;

pub struct Accept {
  fd: RawFd,
  addr: *mut MaybeUninit<SockAddrStorage>,
  len: *mut libc::socklen_t,
}

impl Accept {
  pub fn new(
    fd: RawFd,
    addr: *mut MaybeUninit<SockAddrStorage>,
    len: *mut libc::socklen_t,
  ) -> Self {
    Self { fd, addr, len }
  }
}

impl Operation for Accept {
  impl_result!(fd);

  fn create_entry(&self) -> io_uring::squeue::Entry {
    return io_uring::opcode::Accept::new(
      Fd(self.fd),
      self.addr as *mut libc::sockaddr,
      self.len,
    )
    .build();
  }
}

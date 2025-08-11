use std::{mem::MaybeUninit, os::fd::RawFd};

os_linux! {
  use io_uring::{types::Fd, opcode, squeue};
}
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
  type Output = RawFd;
  type Result = std::io::Result<Self::Output>;

  fn result(&mut self, res: std::io::Result<i32>) -> Self::Result {
    res
  }

  os_linux! {
    const OPCODE: u8 = opcode::Accept::CODE;

    fn create_entry(&self) -> squeue::Entry {
      opcode::Accept::new(
        Fd(self.fd),
        self.addr as *mut libc::sockaddr,
        self.len,
      )
      .build()
    }
  }
  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(accept(self.fd, self.addr as *mut libc::sockaddr, self.len))
  }
}

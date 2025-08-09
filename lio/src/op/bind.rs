use std::{io, os::fd::RawFd};

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
  impl_result!(());

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Bind::CODE;
    fn run_blocking(&self) -> io::Result<i32> {
      syscall!(bind(
        self.fd,
        self.addr.as_ptr().cast::<libc::sockaddr>(),
        self.addr.len() as _
      ))
    }

    fn create_entry(&self) -> io_uring::squeue::Entry {
      let storage = self.addr.as_ptr();
      io_uring::opcode::Bind::new(
        Fd(self.fd),
        storage.cast::<libc::sockaddr>(),
        self.addr.len() as _,
      )
      .build()
    }
  }
}

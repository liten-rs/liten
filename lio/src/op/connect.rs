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
  impl_result!(());

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Connect::CODE;
    fn run_blocking(&self) -> std::io::Result<i32> {
      syscall!(connect(
        self.fd,
        self.addr.as_ptr().cast::<libc::sockaddr>(),
        self.addr.len(),
      ))
    }
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Connect::new(
        Fd(self.fd),
        self.addr.as_ptr().cast::<libc::sockaddr>(),
        self.addr.len(),
      )
      .build()
    }
  }
}

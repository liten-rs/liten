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
  impl_result!(());

  fn create_entry(&self) -> io_uring::squeue::Entry {
    // syscall!(bind(
    //   socket.as_raw_fd(),
    //   sockaddr_ptr.cast::<libc::sockaddr>(),
    //   addr.len() as _,
    // ))?;
    let storage = self.addr.as_ptr();
    io_uring::opcode::Bind::new(
      Fd(self.fd),
      storage.cast::<libc::sockaddr>(),
      self.addr.len() as _,
    )
    .build()
  }
}

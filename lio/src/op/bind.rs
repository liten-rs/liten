use std::{io, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;

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

  #[cfg(linux)]
  const OPCODE: u8 = 56;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    let storage = self.addr.as_ptr();
    io_uring::opcode::Bind::new(
      Fd(self.fd),
      storage.cast::<libc::sockaddr>(),
      self.addr.len() as _,
    )
    .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  fn run_blocking(&self) -> io::Result<i32> {
    syscall!(bind(
      self.fd,
      self.addr.as_ptr().cast::<libc::sockaddr>(),
      self.addr.len()
    ))
  }
}

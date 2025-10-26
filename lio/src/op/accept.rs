use std::{mem::MaybeUninit, os::fd::RawFd};

#[cfg(linux)]
use io_uring::{opcode, squeue, types::Fd};
use socket2::SockAddrStorage;

use crate::op::EventType;

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

  #[cfg(linux)]
  const OPCODE: u8 = 13;

  #[cfg(linux)]
  fn create_entry(&self) -> squeue::Entry {
    opcode::Accept::new(Fd(self.fd), self.addr as *mut libc::sockaddr, self.len)
      .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = Some(EventType::Read);

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    #[cfg(any(
      target_os = "android",
      target_os = "dragonfly",
      target_os = "freebsd",
      target_os = "illumos",
      target_os = "linux",
      target_os = "hurd",
      target_os = "netbsd",
      target_os = "openbsd",
      target_os = "cygwin",
    ))]
    let fd = {
      syscall!(accept4(
        self.fd,
        self.addr as *mut libc::sockaddr,
        self.len,
        libc::SOCK_CLOEXEC
      ))?
    };

    #[cfg(not(any(
      target_os = "android",
      target_os = "dragonfly",
      target_os = "freebsd",
      target_os = "illumos",
      target_os = "linux",
      target_os = "hurd",
      target_os = "netbsd",
      target_os = "openbsd",
      target_os = "cygwin",
    )))]
    let fd = {
      let fd =
        syscall!(accept(self.fd, self.addr as *mut libc::sockaddr, self.len))?;

      syscall!(ioctl(fd, libc::FIOCLEX))?;

      fd
    };

    Ok(fd)
  }
}

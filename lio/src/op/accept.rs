use std::{
  mem::{self},
  os::fd::RawFd,
};

#[cfg(linux)]
use io_uring::{opcode, squeue, types::Fd};

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Accept {
  fd: RawFd,
  addr: libc::sockaddr_un,
  len: libc::socklen_t,
}

unsafe impl Send for Accept {}

impl Accept {
  pub fn new(fd: RawFd) -> Self {
    let sockaddr = unsafe { mem::zeroed::<libc::sockaddr_un>() };
    let len = mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
    Self { fd, addr: sockaddr, len }
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
    opcode::Accept::new(
      Fd(self.fd),
      &self.addr as *const _ as *mut libc::sockaddr,
      &self.len as *const _ as *mut _,
    )
    .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = Some(EventType::Read);

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    let mut socklen = mem::size_of_val(&self.addr) as libc::socklen_t;
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
        &self.addr as *const _ as *mut libc::sockaddr,
        &mut socklen,
        libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK
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
      let fd = syscall!(accept(
        self.fd,
        &self.addr as *const _ as *mut libc::sockaddr,
        &mut socklen
      ))
      .and_then(|socket| {
        // Ensure the socket is closed if either of the `fcntl` calls
        // error below.
        // let s = unsafe { net::UnixStream::from_raw_fd(socket) };
        #[cfg(not(any(target_os = "espidf", target_os = "vita")))]
        syscall!(fcntl(socket, libc::F_SETFD, libc::FD_CLOEXEC))?;

        // See https://github.com/tokio-rs/mio/issues/1450
        #[cfg(not(any(target_os = "espidf", target_os = "vita")))]
        syscall!(fcntl(socket, libc::F_SETFL, libc::O_NONBLOCK))?;

        Ok(socket)
      })?;

      // syscall!(ioctl(fd, libc::FIOCLEX))?;

      fd
    };

    Ok(fd)
  }
}

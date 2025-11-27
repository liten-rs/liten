use std::{
  cell::UnsafeCell,
  mem::{self},
  net::SocketAddr,
  os::fd::RawFd,
};

#[cfg(linux)]
use io_uring::{opcode, squeue, types::Fd};

#[cfg(not(linux))]
use crate::op::EventType;
use crate::op::net_utils::libc_socketaddr_into_std;

use super::Operation;

pub struct Accept {
  fd: RawFd,
  addr: UnsafeCell<libc::sockaddr_storage>,
  len: UnsafeCell<libc::socklen_t>,
}

impl Accept {
  pub(crate) fn new(fd: RawFd) -> Self {
    let addr: libc::sockaddr_storage = unsafe { mem::zeroed() };
    Self {
      fd,
      addr: UnsafeCell::new(addr),
      len: UnsafeCell::new(mem::size_of_val(&addr) as libc::socklen_t),
    }
  }
}

impl Operation for Accept {
  type Output = (RawFd, SocketAddr);
  type Result = std::io::Result<Self::Output>;

  fn result(&mut self, res: std::io::Result<i32>) -> Self::Result {
    Ok((res?, libc_socketaddr_into_std(self.addr.get())?))
  }

  #[cfg(linux)]
  const OPCODE: u8 = 13;

  #[cfg(linux)]
  fn create_entry(&mut self) -> squeue::Entry {
    opcode::Accept::new(
      Fd(self.fd),
      self.addr.get().cast::<libc::sockaddr>(),
      self.len.get(),
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
        self.addr.get() as *mut libc::sockaddr,
        self.len.get() as *mut libc::socklen_t,
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
        self.addr.get() as *mut libc::sockaddr,
        self.len.get() as *mut libc::socklen_t
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

      fd
    };

    Ok(fd)
  }
}

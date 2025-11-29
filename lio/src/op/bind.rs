use std::{cell::UnsafeCell, io, mem, net::SocketAddr, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

use crate::op::{DetachSafe, net_utils::std_socketaddr_into_libc};

use super::Operation;

pub struct Bind {
  fd: RawFd,
  addr: UnsafeCell<libc::sockaddr_storage>, // addr: SocketAddr,
}

unsafe impl DetachSafe for Bind {}

impl Bind {
  pub(crate) fn new(fd: RawFd, addr: SocketAddr) -> Self {
    Self { fd, addr: UnsafeCell::new(std_socketaddr_into_libc(addr)) }
  }
}

impl Operation for Bind {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 56;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::Bind::new(
      Fd(self.fd),
      self.addr.get().cast(),
      mem::size_of_val(&self.addr) as libc::socklen_t,
    )
    .build()
  }

  impl_no_readyness!();

  fn run_blocking(&self) -> io::Result<i32> {
    let storage = unsafe { &*self.addr.get() };
    let addrlen = if storage.ss_family == libc::AF_INET as libc::sa_family_t {
      mem::size_of::<libc::sockaddr_in>()
    } else if storage.ss_family == libc::AF_INET6 as libc::sa_family_t {
      mem::size_of::<libc::sockaddr_in6>()
    } else {
      mem::size_of::<libc::sockaddr_storage>()
    };

    syscall!(bind(self.fd, self.addr.get().cast(), addrlen as libc::socklen_t,))
  }
}

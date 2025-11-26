use std::{
  cell::UnsafeCell,
  io, mem,
  net::{SocketAddr, SocketAddrV4, SocketAddrV6},
  os::fd::RawFd,
  ptr,
};

#[cfg(linux)]
use io_uring::types::Fd;
use socket2::Domain;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Bind {
  fd: RawFd,
  addr: UnsafeCell<libc::sockaddr_storage>, // addr: SocketAddr,
}
impl Bind {
  pub(crate) fn new(fd: RawFd, addr: SocketAddr) -> Self {
    let mut storage: libc::sockaddr_storage = unsafe { mem::zeroed() };

    match addr {
      SocketAddr::V4(v4) => unsafe {
        // We calculate the size of the IPv4 address structure
        let size = mem::size_of::<libc::sockaddr_in>();

        // We copy the bytes from the source pointer (&v4)
        // to the destination pointer (&mut storage)
        ptr::copy_nonoverlapping(
          &Self::into_addr(v4) as *const _ as *const u8,
          &mut storage as *mut _ as *mut u8,
          size,
        );
      },
      SocketAddr::V6(v6) => unsafe {
        // We calculate the size of the IPv6 address structure
        let size = mem::size_of::<libc::sockaddr_in6>();

        // We copy the bytes from the source pointer (&v6)
        // to the destination pointer (&mut storage)
        ptr::copy_nonoverlapping(
          &Self::into_addr6(v6) as *const _ as *const u8,
          &mut storage as *mut _ as *mut u8,
          size,
        );
      },
    };

    Self { fd, addr: UnsafeCell::new(storage) }
  }

  fn into_addr(addr: SocketAddrV4) -> libc::sockaddr_in {
    let mut _addr: libc::sockaddr_in = unsafe { mem::zeroed() };

    let family: i32 = Domain::IPV4.into();
    _addr.sin_family = family as libc::sa_family_t;
    _addr.sin_port = addr.port().to_be();
    _addr.sin_addr =
      libc::in_addr { s_addr: u32::from_be_bytes(addr.ip().octets()).to_be() };

    _addr
  }

  fn into_addr6(addr: SocketAddrV6) -> libc::sockaddr_in6 {
    let mut _addr: libc::sockaddr_in6 = unsafe { mem::zeroed() };

    let family: i32 = Domain::for_address(addr.into()).into();
    _addr.sin6_family = family as libc::sa_family_t;
    _addr.sin6_port = addr.port().to_be();

    _addr.sin6_addr = libc::in6_addr { s6_addr: addr.ip().octets() };

    _addr
  }
}

impl Operation for Bind {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 56;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Bind::new(
      Fd(self.fd),
      self.addr.get() as *const libc::sockaddr,
      mem::size_of_val(&self.addr) as libc::socklen_t,
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
      self.addr.get() as *const libc::sockaddr,
      mem::size_of_val(&self.addr) as libc::socklen_t,
    ))
  }
}

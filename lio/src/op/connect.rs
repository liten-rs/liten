use std::cell::UnsafeCell;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{mem, ptr};

#[cfg(linux)]
use io_uring::types::Fd;
use socket2::Domain;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Connect {
  fd: RawFd,
  addr: UnsafeCell<libc::sockaddr_storage>,
  connect_called: AtomicBool,
}

impl Connect {
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

    Self {
      fd,
      addr: UnsafeCell::new(storage),
      connect_called: AtomicBool::new(false),
    }
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

impl Operation for Connect {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 16;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Connect::new(
      Fd(self.fd),
      self.addr.get() as *const libc::sockaddr,
      mem::size_of_val(&self.addr) as libc::socklen_t,
    )
    .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = Some(EventType::Write);

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    let result = syscall!(connect(
      self.fd,
      self.addr.get() as *const libc::sockaddr,
      mem::size_of_val(&self.addr) as libc::socklen_t,
    ));

    // Track if this is the first connect() call for this operation
    let is_first_call = !self.connect_called.swap(true, Ordering::SeqCst);

    if let Err(ref err) = result {
      if let Some(errno) = err.raw_os_error() {
        // - If this is the first connect() call: socket was already connected (error)
        // - If this is a subsequent call: connection just completed (success)
        if errno == libc::EISCONN {
          if is_first_call {
            // First connect() returned EISCONN = socket was already connected
            return Err(std::io::Error::from_raw_os_error(56));
          } else {
            // Subsequent connect() returned EISCONN = connection completed
            return Ok(0);
          }
        }

        if errno == libc::EALREADY {
          return Err(std::io::Error::from_raw_os_error(libc::EINPROGRESS));
        }
      }
    };
    result
  }
}

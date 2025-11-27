use std::cell::UnsafeCell;
use std::mem;
use std::net::SocketAddr;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;
use crate::op::net_utils::std_socketaddr_into_libc;

use super::Operation;

pub struct Connect {
  fd: RawFd,
  addr: UnsafeCell<libc::sockaddr_storage>,
  connect_called: AtomicBool,
}

impl Connect {
  pub(crate) fn new(fd: RawFd, addr: SocketAddr) -> Self {
    Self {
      fd,
      addr: UnsafeCell::new(std_socketaddr_into_libc(addr)),
      connect_called: AtomicBool::new(false),
    }
  }
}

impl Operation for Connect {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 16;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::Connect::new(
      Fd(self.fd),
      self.addr.get().cast(),
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
    let storage = unsafe { &*self.addr.get() };
    let addrlen = if storage.ss_family == libc::AF_INET as libc::sa_family_t {
      mem::size_of::<libc::sockaddr_in>()
    } else if storage.ss_family == libc::AF_INET6 as libc::sa_family_t {
      mem::size_of::<libc::sockaddr_in6>()
    } else {
      mem::size_of::<libc::sockaddr_storage>()
    };

    let result = syscall!(connect(
      self.fd,
      self.addr.get().cast(),
      addrlen as libc::socklen_t,
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

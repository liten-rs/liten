use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::types::Fd;
use socket2::SockAddr;

#[cfg(not(linux))]
use crate::op::EventType;
#[cfg(not(linux))]
use crate::shuttle::sync::atomic::{AtomicBool, Ordering};

use super::Operation;

pub struct Connect {
  fd: RawFd,
  addr: SockAddr,
  #[cfg(not(linux))]
  connect_called: AtomicBool,
}

impl Connect {
  pub fn new(fd: RawFd, addr: SockAddr) -> Self {
    Self {
      fd,
      addr,
      #[cfg(not(linux))]
      connect_called: AtomicBool::new(false),
    }
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
      self.addr.as_ptr().cast::<libc::sockaddr>(),
      self.addr.len(),
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
      self.addr.as_ptr().cast::<libc::sockaddr>(),
      self.addr.len(),
    ));

    // Handle platform-specific connect() behavior for non-blocking sockets
    #[cfg(not(linux))]
    {
      // Track if this is the first connect() call for this operation
      let is_first_call = !self.connect_called.swap(true, Ordering::SeqCst);

      if let Err(ref err) = result {
        if let Some(errno) = err.raw_os_error() {
          match errno {
            // EISCONN: Socket is already connected
            // - If this is the first connect() call: socket was already connected (error)
            // - If this is a subsequent call: connection just completed (success)
            56 => {
              if is_first_call {
                // First connect() returned EISCONN = socket was already connected
                return Err(std::io::Error::from_raw_os_error(56));
              } else {
                // Subsequent connect() returned EISCONN = connection completed
                return Ok(0);
              }
            }
            // EALREADY: Previous connection attempt still in progress
            // Return EINPROGRESS to keep polling
            37 => {
              return Err(std::io::Error::from_raw_os_error(libc::EINPROGRESS));
            }
            _ => {}
          }
        }
      }
    }

    result
  }
}

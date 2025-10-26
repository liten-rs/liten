use std::io;
#[cfg(not(linux))]
use std::os::fd::RawFd;

use crate::op::EventType;

use super::Operation;

pub struct Socket {
  domain: socket2::Domain,
  ty: socket2::Type,
  proto: Option<socket2::Protocol>,
}

impl Socket {
  pub fn new(
    domain: socket2::Domain,
    ty: socket2::Type,
    proto: Option<socket2::Protocol>,
  ) -> Self {
    Self { domain, ty, proto }
  }
}

impl Operation for Socket {
  #[cfg(unix)]
  type Output = std::os::fd::RawFd;

  #[cfg(unix)]
  type Result = std::io::Result<Self::Output>;

  #[doc = r" File descriptor returned from the operation."]
  fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
    fd
  }

  #[cfg(linux)]
  const OPCODE: u8 = 45;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Socket::new(
      self.domain.into(),
      self.ty.into(),
      self.proto.unwrap_or(0.into()).into(),
    )
    .build()
  }

  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  fn run_blocking(&self) -> io::Result<i32> {
    let fd = syscall!(socket(
      self.domain.into(),
      self.ty.into(),
      self.proto.unwrap_or(0.into()).into()
    ))
    .map(|t| t as i32)?;

    // Remove blocking by kernel if no result is available. Linux uses io-uring so it doesn't
    // count.
    #[cfg(not(linux))]
    syscall!(ioctl(fd, libc::FIONBIO, &mut 1))?;

    let opt: i32 = 1;
    syscall!(setsockopt(
      fd,
      libc::SOL_SOCKET,
      libc::SO_REUSEADDR,
      &opt as *const i32 as *const libc::c_void,
      4
    ))?;

    Ok(fd)
  }
}

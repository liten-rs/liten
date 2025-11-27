use std::{io, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

use crate::BufResult;
#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Recv {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  flags: i32,
}

impl Recv {
  pub(crate) fn new(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> Self {
    Self { fd, buf: Some(buf), flags: flags.unwrap_or(0) }
  }
}

impl Operation for Recv {
  type Output = i32;
  type Result = BufResult<Self::Output, Vec<u8>>;

  #[cfg(linux)]
  const OPCODE: u8 = 27;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    if let Some(ref mut buf) = self.buf {
      io_uring::opcode::Recv::new(
        Fd(self.fd),
        buf.as_mut_ptr(),
        buf.len() as u32,
      )
      .flags(self.flags)
      .build()
    } else {
      unreachable!()
    }
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = Some(EventType::Read);

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  fn run_blocking(&self) -> io::Result<i32> {
    let buf = self.buf.as_ref().unwrap();
    syscall!(recv(self.fd, buf.as_ptr() as *mut _, buf.len(), self.flags))
      .map(|t| t as i32)
  }

  fn result(&mut self, _ret: io::Result<i32>) -> Self::Result {
    let buf = self.buf.take().expect("ran Recv::result more than once.");

    (_ret, buf)
  }
}

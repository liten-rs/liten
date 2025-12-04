use std::{io, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;
use crate::{BufResult, op::DetachSafe};

use super::Operation;

pub struct Send {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  flags: i32,
}

unsafe impl DetachSafe for Send {}

impl Send {
  pub(crate) fn new(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> Self {
    assert!((buf.len()) <= u32::MAX as usize);
    Self { fd, buf: Some(buf), flags: flags.unwrap_or(0) }
  }
}

impl Operation for Send {
  type Result = BufResult<i32, Vec<u8>>;

  #[cfg(linux)]
  const OPCODE: u8 = 26;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    let buf = self.buf.as_ref().unwrap();
    io_uring::opcode::Send::new(Fd(self.fd), buf.as_ptr(), buf.len() as u32)
      .flags(self.flags)
      .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = Some(EventType::Write);

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    Some(self.fd)
  }

  #[cfg(not(linux))]
  fn run_blocking(&self) -> io::Result<i32> {
    let buf = self.buf.as_ref().unwrap();
    syscall!(send(self.fd, buf.as_ptr() as *mut _, buf.len(), self.flags))
      .map(|t| t as i32)
  }
  fn result(&mut self, _ret: std::io::Result<i32>) -> Self::Result {
    let buf = self.buf.take().expect("ran Recv::result more than once.");

    (_ret, buf)
  }
}

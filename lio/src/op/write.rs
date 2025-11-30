use super::Operation;
use crate::{BufResult, op::DetachSafe};

#[cfg(not(linux))]
use crate::op::EventType;

#[cfg(linux)]
use io_uring::types::Fd;

use std::os::fd::RawFd;

pub struct Write {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  offset: i64,
}

unsafe impl DetachSafe for Write {}

impl Write {
  pub(crate) fn new(fd: RawFd, buf: Vec<u8>, offset: i64) -> Write {
    assert!((buf.len()) <= u32::MAX as usize);
    Self { fd, buf: Some(buf), offset }
  }
}

impl Operation for Write {
  type Output = i32;
  type Result = BufResult<Self::Output, Vec<u8>>;

  #[cfg(linux)]
  const OPCODE: u8 = 23;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    let buf = self.buf.as_ref().unwrap();
    io_uring::opcode::Write::new(Fd(self.fd), buf.as_ptr(), buf.len() as u32)
      .offset(self.offset as u64)
      .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(not(linux))]
  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(pwrite(
      self.fd,
      self.buf.as_ref().unwrap().as_ptr() as *const _,
      self.buf.as_ref().unwrap().len() as usize,
      self.offset
    ))
    .map(|u| u as i32)
  }

  fn result(&mut self, _ret: std::io::Result<i32>) -> Self::Result {
    let buf = self.buf.take().expect("ran Recv::result more than once.");

    (_ret, buf)
  }
}

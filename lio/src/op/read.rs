use std::{io, os::fd::RawFd};

#[cfg(linux)]
use io_uring::types::Fd;

use crate::BufResult;

use super::Operation;

pub struct Read {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  offset: i64,
}

impl Read {
  pub fn new(fd: RawFd, mem: Vec<u8>, offset: i64) -> Self {
    Self { fd, buf: Some(mem), offset }
  }
}

impl Operation for Read {
  #[cfg(linux)]
  const OPCODE: u8 = 22;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    if let Some(ref buf) = self.buf {
      io_uring::opcode::Read::new(
        Fd(self.fd),
        buf.as_ptr() as *mut _,
        buf.len() as u32,
      )
      .offset(self.offset as u64)
      .build()
    } else {
      unreachable!()
    }
  }
  type Output = i32;
  type Result = BufResult<Self::Output, Vec<u8>>;

  fn run_blocking(&self) -> io::Result<i32> {
    let buf = self.buf.as_ref().unwrap();
    syscall!(pread(self.fd, buf.as_ptr() as *mut _, buf.len(), self.offset))
      .map(|t| t as i32)
  }
  fn result(&mut self, _ret: io::Result<i32>) -> Self::Result {
    let buf = self.buf.take().expect("ran Recv::result more than once.");

    match _ret {
      Ok(ret) => (Ok(ret), buf),
      Err(err) => (Err(err), buf),
    }
  }
}

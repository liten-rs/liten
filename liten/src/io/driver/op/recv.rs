use std::{io, os::fd::RawFd};

use io_uring::types::Fd;

use crate::io::BufResult;

use super::Operation;

pub struct Recv {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  flags: i32,
}

impl Recv {
  pub fn new(fd: RawFd, length: u32, flags: Option<i32>) -> Self {
    let mut mem = Vec::with_capacity(length as usize);

    for _ in 0..length as usize {
      mem.push(0);
    }
    Self { fd, buf: Some(mem), flags: flags.unwrap_or(0) }
  }
}

impl Operation for Recv {
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Recv::new(
      Fd(self.fd),
      self.buf.as_ref().unwrap().as_ptr() as *mut _,
      self.buf.as_ref().unwrap().len() as u32,
    )
    .flags(self.flags)
    .build()
  }
  type Output = i32;
  type Result = BufResult<Self::Output, io::Error, Vec<u8>>;
  fn result(&mut self, _ret: io::Result<i32>) -> Self::Result {
    let buf = self.buf.take().expect("ran Recv::result more than once.");

    match _ret {
      Ok(ret) => (Ok(ret), buf),
      Err(err) => (Err(err), buf),
    }
  }
}

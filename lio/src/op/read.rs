use std::{io, os::fd::RawFd};

use io_uring::types::Fd;

use crate::BufResult;

use super::Operation;

pub struct Read {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  offset: u64,
}

impl Read {
  pub fn new(fd: RawFd, mem: Vec<u8>, offset: u64) -> Self {
    Self { fd, buf: Some(mem), offset }
  }
}

impl Operation for Read {
  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Read::CODE;
    fn create_entry(&self) -> io_uring::squeue::Entry {
      if let Some(ref buf) = self.buf {
        io_uring::opcode::Read::new(
          Fd(self.fd),
          buf.as_ptr() as *mut _,
          buf.len() as u32,
        )
        .offset(self.offset)
        .build()
      } else {
        unreachable!()
      }
    }
    type Output = i32;
    type Result = BufResult<Self::Output, Vec<u8>>;
    fn run_blocking(&self) -> io::Result<i32> {
      let buf = self.buf.as_ref().unwrap();
      syscall!(read(self.fd, buf.as_ptr() as *mut _, buf.len())).map(|t| t as i32)
    }
    fn result(&mut self, _ret: io::Result<i32>) -> Self::Result {
      let buf = self.buf.take().expect("ran Recv::result more than once.");

      match _ret {
        Ok(ret) => (Ok(ret), buf),
        Err(err) => (Err(err), buf),
      }
    }
  }
}

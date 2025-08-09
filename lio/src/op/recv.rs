use std::{io, os::fd::RawFd};

use io_uring::types::Fd;

use crate::BufResult;

use super::Operation;

pub struct Recv {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  flags: i32,
}

impl Recv {
  pub fn new(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> Self {
    Self { fd, buf: Some(buf), flags: flags.unwrap_or(0) }
  }
}

impl Operation for Recv {
  type Output = i32;
  type Result = BufResult<Self::Output, Vec<u8>>;
  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Recv::CODE;

    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Recv::new(
        Fd(self.fd),
        self.buf.as_ref().unwrap().as_ptr() as *mut _,
        self.buf.as_ref().unwrap().len() as u32,
      )
      .flags(self.flags)
      .build()
    }

    fn run_blocking(&self) -> io::Result<i32> {
      let buf = self.buf.as_ref().unwrap();
      syscall!(recv(self.fd, buf.as_ptr() as *mut _, buf.len(), 0))
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
}

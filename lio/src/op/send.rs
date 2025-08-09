use std::{io, os::fd::RawFd};

use io_uring::types::Fd;

use crate::BufResult;

use super::Operation;

pub struct Send {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  flags: i32,
}

impl Send {
  pub fn new(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> Self {
    assert!((buf.len()) <= u32::MAX as usize);
    Self { fd, buf: Some(buf), flags: flags.unwrap_or(0) }
  }
}

impl Operation for Send {
  type Output = i32;
  type Result = BufResult<Self::Output, Vec<u8>>;

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Send::CODE;
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Send::new(
        Fd(self.fd),
        self.buf.as_ref().unwrap().as_ptr(),
        self.buf.as_ref().unwrap().len() as u32,
      )
      .flags(self.flags)
      .build()
    }


    fn run_blocking(&self) -> io::Result<i32> {
      let buf = self.buf.as_ref().unwrap();
      syscall!(send(self.fd, buf.as_ptr() as *mut _, buf.len(), 0))
        .map(|t| t as i32)
    }
    fn result(&mut self, _ret: std::io::Result<i32>) -> Self::Result {
      let buf = self.buf.take().expect("ran Recv::result more than once.");

      match _ret {
        Ok(ret) => (Ok(ret), buf),
        Err(err) => (Err(err), buf),
      }
    }
  }
}

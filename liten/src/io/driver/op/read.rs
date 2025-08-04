use std::os::fd::RawFd;

use io_uring::types::Fd;

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
  type Output = Vec<u8>;
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
  fn result(&mut self) -> Self::Output {
    self.buf.take().expect("ran Read::result more than once.")
  }
}

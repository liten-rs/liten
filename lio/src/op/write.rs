use super::Operation;
use crate::BufResult;
use io_uring::types::Fd;
use std::os::fd::RawFd;

pub struct Write {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  offset: u64,
}

impl Write {
  pub fn new(fd: RawFd, buf: Vec<u8>, offset: u64) -> Write {
    assert!((buf.len()) <= u32::MAX as usize);
    Self { fd, buf: Some(buf), offset }
  }
}

impl Operation for Write {
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Write::new(
      Fd(self.fd),
      self.buf.as_ref().unwrap().as_ptr(),
      self.buf.as_ref().unwrap().len() as u32,
    )
    .offset(self.offset)
    .build()
  }

  type Output = i32;

  type Result = BufResult<Self::Output, Vec<u8>>;

  fn result(&mut self, _ret: std::io::Result<i32>) -> Self::Result {
    let buf = self.buf.take().expect("ran Recv::result more than once.");

    match _ret {
      Ok(ret) => (Ok(ret), buf),
      Err(err) => (Err(err), buf),
    }
  }
}

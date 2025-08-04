use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Send {
  fd: RawFd,
  buf: Vec<u8>,
  flags: i32,
}

impl Send {
  pub fn new(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> Self {
    assert!((buf.len()) <= u32::MAX as usize);
    Self { fd, buf, flags: flags.unwrap_or(0) }
  }
}

impl Operation for Send {
  type Output = ();
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Send::new(
      Fd(self.fd),
      self.buf.as_ptr(),
      self.buf.len() as u32,
    )
    .flags(self.flags)
    .build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

use std::os::fd::RawFd;

use io_uring::types::Fd;

use super::Operation;

pub struct Write {
  fd: RawFd,
  buf: Box<[u8]>,
  offset: u64,
}

impl Write {
  pub fn new(fd: RawFd, buf: Box<[u8]>, offset: u64) -> Write {
    assert!((buf.len()) <= u32::MAX as usize);
    Self { fd, buf, offset }
  }
}

impl Operation for Write {
  type Output = ();
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Write::new(
      Fd(self.fd),
      self.buf.as_ptr(),
      self.buf.len() as u32,
    )
    .offset(self.offset)
    .build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

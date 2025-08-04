use std::os::fd::RawFd;

use io_uring::types::Fd;

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
  type Output = Vec<u8>;
  fn create_entry(&self) -> io_uring::squeue::Entry {
    if let Some(ref buf) = self.buf {
      io_uring::opcode::Recv::new(
        Fd(self.fd),
        buf.as_ptr() as *mut _,
        buf.len() as u32,
      )
      .flags(self.flags)
      .build()
    } else {
      unreachable!()
    }
  }
  fn result(&mut self) -> Self::Output {
    self.buf.take().expect("ran Recv::result more than once.")
  }
}

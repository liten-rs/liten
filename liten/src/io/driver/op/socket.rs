use std::os::fd::RawFd;

use super::Operation;

pub struct Socket {
  domain: i32,
  ty: i32,
  proto: i32,
}

impl Socket {
  pub fn new(domain: i32, ty: i32, proto: i32) -> Self {
    Self { domain, ty, proto }
  }
}

impl Operation for Socket {
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Socket::new(self.domain, self.ty, self.proto).build()
  }
  impl_result!(fd);
}

use super::Operation;

pub struct Socket {
  domain: socket2::Domain,
  ty: socket2::Type,
  proto: Option<socket2::Protocol>,
}

impl Socket {
  pub fn new(
    domain: socket2::Domain,
    ty: socket2::Type,
    proto: Option<socket2::Protocol>,
  ) -> Self {
    Self { domain, ty, proto }
  }
}

impl Operation for Socket {
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Socket::new(
      self.domain.into(),
      self.ty.into(),
      self.proto.unwrap_or(0.into()).into(),
    )
    .build()
  }
  impl_result!(fd);
}

use std::io;

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
  impl_result!(fd);

  os_linux! {
    const OPCODE: u8 = io_uring::opcode::Socket::CODE;
    fn create_entry(&self) -> io_uring::squeue::Entry {
      io_uring::opcode::Socket::new(
        self.domain.into(),
        self.ty.into(),
        self.proto.unwrap_or(0.into()).into(),
      )
      .build()
    }
  }
  fn run_blocking(&self) -> io::Result<i32> {
    let result = syscall!(socket(
      self.domain.into(),
      self.ty.into(),
      self.proto.unwrap_or(0.into()).into()
    ))
    .map(|t| t as i32)?;

    // Remove blocking by kernel if no result is available. Linux uses io-uring so it doesn't
    // count.
    #[cfg(not_linux)]
    syscall!(ioctl(result, libc::FIONBIO, &mut 1))?;

    Ok(result)
  }
}

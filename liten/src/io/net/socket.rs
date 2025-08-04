use std::{io, mem::MaybeUninit, os::fd::RawFd};

use socket2::{Domain, SockAddr, SockAddrStorage, Type};

use crate::io::Driver;

pub struct Socket {
  fd: RawFd,
}

impl Socket {
  pub async fn new(addr: Domain, ty: Type) -> io::Result<Self> {
    let fd = Driver::socket(addr.into(), ty.into(), 0).await?;
    Ok(Self { fd })
  }

  // FIXME: i think something wrong here?
  pub async fn bind(&self, addr: SockAddr) -> io::Result<()> {
    Driver::bind(self.fd, addr).await
  }

  pub async fn listen(&self) -> io::Result<()> {
    Driver::listen(self.fd, 128).await
  }

  pub async fn accept(&self) -> io::Result<(RawFd, SockAddr)> {
    let mut storage: MaybeUninit<SockAddrStorage> = MaybeUninit::uninit();
    let mut len = size_of_val(&storage) as libc::socklen_t;
    let fd =
      Driver::accept(self.fd, storage.as_mut_ptr() as *mut _, &mut len).await?;

    Ok((fd, unsafe { SockAddr::new(storage.assume_init(), len) }))
  }

  pub async fn send(&self, buf: Vec<u8>) -> io::Result<i32> {
    Driver::send(self.fd, buf, None).await
  }

  pub async fn recv(&self, len: u32) -> io::Result<Vec<u8>> {
    Driver::recv(self.fd, len, None).await
  }

  pub async fn connect(&self, addr: SockAddr) -> io::Result<RawFd> {
    Driver::connect(self.fd, addr).await
  }
}

impl Drop for Socket {
  fn drop(&mut self) {
    if unsafe { libc::close(self.fd) } == -1 {
      panic!("error closing socket: {:#?}", io::Error::last_os_error());
    }
  }
}

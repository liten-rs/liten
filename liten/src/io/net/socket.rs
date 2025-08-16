use std::{future::Future, io, mem::MaybeUninit, os::fd::RawFd};

use socket2::{SockAddr, SockAddrStorage, Type};

use crate::io::{AsyncRead, AsyncWrite};

pub struct Socket {
  fd: RawFd,
}

fn setsocketopt<T>(
  fd: RawFd,
  level: libc::c_int,
  value: libc::c_int,
  flag: T,
) -> io::Result<i32> {
  let res = unsafe {
    libc::setsockopt(
      fd,
      level,
      value,
      (&raw const flag) as *const _,
      std::mem::size_of::<T>() as libc::socklen_t,
    )
  };
  if res == -1 {
    Err(std::io::Error::last_os_error())
  } else {
    Ok(res)
  }
}

impl Socket {
  unsafe fn from_raw_fd(fd: RawFd) -> Self {
    Self { fd }
  }

  pub async fn bind(addr: SockAddr, ty: Type) -> io::Result<Self> {
    let fd =
      lio::socket(addr.domain(), ty, Some(libc::IPPROTO_TCP.into())).await?;
    lio::bind(fd, addr).await?;

    // https://github.com/rust-lang/rust/blob/18eeac04fc5c2a4c4a8020dbdf1c652077ad0e4e/library/std/src/sys/net/connection/socket.rs#L516-L518
    #[cfg(not(windows))]
    setsocketopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, 1)?;

    Ok(Self { fd })
  }
  pub async fn connect(addr: SockAddr, ty: Type) -> io::Result<Self> {
    let fd = lio::socket(addr.domain(), ty, None).await?;
    lio::connect(fd, addr).await?;

    Ok(Self { fd })
  }

  pub fn listen(&self) -> impl Future<Output = io::Result<()>> {
    lio::listen(self.fd)
  }

  pub async fn accept(&self) -> io::Result<(Socket, SockAddr)> {
    let mut storage: MaybeUninit<SockAddrStorage> = MaybeUninit::uninit();
    let mut len = size_of_val(&storage) as libc::socklen_t;
    let fd =
      lio::accept(self.fd, storage.as_mut_ptr() as *mut _, &mut len).await?;

    let addr = unsafe { SockAddr::new(storage.assume_init(), len) };

    Ok((unsafe { Socket::from_raw_fd(fd) }, addr))
  }
}

impl AsyncRead for Socket {
  async fn read(
    &mut self,
    buf: Vec<u8>,
  ) -> crate::io::BufResult<usize, Vec<u8>> {
    let (result, buf) = lio::recv(self.fd, buf, None).await;

    (result.map(|bytes_read| bytes_read as usize), buf)
  }
}

impl AsyncWrite for Socket {
  async fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> crate::io::BufResult<usize, Vec<u8>> {
    let (result, buf) = lio::send(self.fd, buf, None).await;

    (result.map(|bytes_read| bytes_read as usize), buf)
  }
  async fn flush(&mut self) -> io::Result<()> {
    // net sockets cannot flush sockets
    Ok(())
  }
}

impl Drop for Socket {
  fn drop(&mut self) {
    lio::close(self.fd).detach();
  }
}

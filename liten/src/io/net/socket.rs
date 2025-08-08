use std::{
  io,
  mem::{self, MaybeUninit},
  os::fd::{AsRawFd, RawFd},
};

use socket2::{SockAddr, SockAddrStorage, Type};

use crate::io::{AsyncRead, AsyncWrite};

pub struct Socket {
  fd: RawFd,
}

macro_rules! syscall {
  ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
      #[allow(unused_unsafe)]
      let res = unsafe { libc::$fn($($arg, )*) };
      if res == -1 {
          Err(std::io::Error::last_os_error())
      } else {
          Ok(res)
      }
  }};
}

impl Socket {
  unsafe fn from_raw_fd(fd: RawFd) -> Self {
    Self { fd }
  }

  pub async fn bind(addr: SockAddr, ty: Type) -> io::Result<Self> {
    let socket = socket2::Socket::new(addr.domain(), ty, None)?;

    let sockaddr_ptr = &addr as *const SockAddr;

    // Once WSL moves to 6.11+ when bind is supported.
    // lio::bind(fd, addr).await?;

    // Instead of this:
    syscall!(bind(
      socket.as_raw_fd(),
      sockaddr_ptr.cast::<libc::sockaddr>(),
      addr.len() as _,
    ))?;

    let fd = socket.as_raw_fd();

    mem::forget(socket);

    let flag = 1;
    let result = unsafe {
      libc::setsockopt(
        fd,
        libc::SOL_SOCKET,
        libc::SO_REUSEADDR | libc::SO_REUSEPORT,
        &flag as *const _ as *const libc::c_void,
        std::mem::size_of_val(&flag) as libc::socklen_t,
      )
    };

    if result < 0 {
      return Err(std::io::Error::from_raw_os_error(result));
    };

    Ok(Self { fd })
  }
  pub async fn connect(addr: SockAddr, ty: Type) -> io::Result<Self> {
    let fd = lio::socket(addr.domain(), ty, None).await?;
    lio::connect(fd, addr).await?;

    Ok(Self { fd })
  }

  pub async fn listen(&self, backlog: i32) -> io::Result<()> {
    // Once WSL moves to 6.11+ when listen is supported.
    // lio::listen(fd, backlog).await?;
    //
    // Instead of this:
    syscall!(listen(self.fd, backlog)).map(|_| ())
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
    lio::close(self.fd).detatch();
  }
}

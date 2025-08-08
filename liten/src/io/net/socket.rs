use std::{
  future::Future,
  io,
  mem::{self, MaybeUninit},
  net::{SocketAddr, ToSocketAddrs},
  os::fd::{AsRawFd, FromRawFd, RawFd},
};

use socket2::{SockAddr, SockAddrStorage, Socket as Socket2, Type};

use crate::future::{FutureExt, Stream};
use crate::io::Driver;

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
    let socket = Socket2::from_raw_fd(fd);
    let fd = socket.as_raw_fd();
    mem::forget(socket);

    Self { fd }
  }

  pub async fn bind(addr: SockAddr, ty: Type) -> io::Result<Self> {
    let socket = socket2::Socket::new(addr.domain(), ty, None)?;

    let sockaddr_ptr = &addr as *const SockAddr;

    // Once WSL moves to 6.11+ when bind is supported.
    // Driver::bind(fd, addr).await?;

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
    let socket = socket2::Socket::new(addr.domain(), ty, None)?;
    let fd = socket.as_raw_fd();
    Driver::connect(fd, addr).await?;

    mem::forget(socket);

    Ok(Self { fd })
  }

  pub async fn listen(&self, backlog: i32) -> io::Result<()> {
    // Once WSL moves to 6.11+ when bind is supported.
    // Driver::listen(fd, backlog).await?;
    //
    // Instead of this:
    syscall!(listen(self.fd, backlog)).map(|_| ())
  }

  pub async fn accept(&self) -> io::Result<(Socket, SockAddr)> {
    let mut storage: MaybeUninit<SockAddrStorage> = MaybeUninit::uninit();
    let mut len = size_of_val(&storage) as libc::socklen_t;
    let fd =
      Driver::accept(self.fd, storage.as_mut_ptr() as *mut _, &mut len).await?;

    let addr = unsafe { SockAddr::new(storage.assume_init(), len) };

    Ok((unsafe { Socket::from_raw_fd(fd) }, addr))
  }

  pub async fn write(&self, buf: Vec<u8>) -> io::Result<(i32, Vec<u8>)> {
    let (res, buf) = Driver::send(self.fd, buf, None).await;
    Ok((res?, buf))
  }

  pub async fn read(&self, len: u32) -> io::Result<(i32, Vec<u8>)> {
    let (err_or_bytes_ret, buf) = Driver::recv(self.fd, len, None).await;
    Ok((err_or_bytes_ret?, buf))
  }
}

impl Drop for Socket {
  fn drop(&mut self) {
    Driver::close(self.fd).detatch();
  }
}

pub struct TcpListener(Socket);

impl TcpListener {
  pub async fn bind<T: ToSocketAddrs>(addr: T) -> io::Result<Self> {
    let addr = SockAddr::from(addr.to_socket_addrs()?.next().unwrap());
    let socket = Socket::bind(addr, Type::STREAM).await?;

    socket.listen(512).await?;

    Ok(TcpListener(socket))
  }

  pub async fn accept(&self) -> io::Result<(TcpStream, SockAddr)> {
    let (socket, addr) = self.0.accept().await?;

    Ok((TcpStream(socket), addr))
  }
}

impl Stream for TcpListener {
  type Item = io::Result<(TcpStream, SockAddr)>;
  fn next(&self) -> impl Future<Output = Option<Self::Item>> {
    self.accept().map(Some)
  }
}

pub struct TcpStream(Socket);

impl TcpStream {
  pub async fn connect(addr: SocketAddr) -> io::Result<Self> {
    let socket = Socket::connect(addr.into(), Type::STREAM).await?;
    Ok(Self(socket))
  }
  pub fn read(
    &self,
    count: u32,
  ) -> impl Future<Output = io::Result<(i32, Vec<u8>)>> + '_ {
    self.0.read(count)
  }

  pub fn write(
    &self,
    bytes: Vec<u8>,
  ) -> impl Future<Output = io::Result<(i32, Vec<u8>)>> + '_ {
    self.0.write(bytes)
  }
}

use std::io::{self};
use std::net::{SocketAddr, ToSocketAddrs};
use std::os::fd::{FromRawFd, RawFd};

use socket2::{Domain, Protocol, Type};

pub struct Fd(RawFd);

impl FromRawFd for Fd {
  unsafe fn from_raw_fd(fd: RawFd) -> Self {
    Self(fd)
  }
}

impl Drop for Fd {
  fn drop(&mut self) {
    lio::close(self.0).detach();
  }
}

pub struct TcpListener(Socket);

impl From<Socket> for TcpListener {
  fn from(value: Socket) -> Self {
    TcpListener(value)
  }
}

impl TcpListener {
  pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
    let mut iter = addr.to_socket_addrs()?;

    while let Some(value) = iter.next() {
      let domain = match value {
        SocketAddr::V6(_) => Domain::IPV6,
        SocketAddr::V4(_) => Domain::IPV4,
      };
      let socket =
        Socket::new(domain, Type::STREAM, Some(Protocol::TCP)).await?;

      socket.bind(value).await?;
      socket.listen().await?;
      return Ok(TcpListener(socket));
    }

    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "could not resolve to any addresses",
    ));
  }

  pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
    let (rawfd, addr) = lio::accept(self.0.0.0).await?;

    let socket = Socket::from(unsafe { Fd::from_raw_fd(rawfd) });

    Ok((TcpStream(socket), addr))
  }
}

pub struct TcpStream(Socket);

impl TcpStream {
  pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
    let mut iter = addr.to_socket_addrs()?;

    while let Some(value) = iter.next() {
      let domain = match value {
        SocketAddr::V6(_) => Domain::IPV6,
        SocketAddr::V4(_) => Domain::IPV4,
      };
      let socket = Socket::new(domain, Type::STREAM, None).await?;

      socket.connect(value).await?;

      return Ok(TcpStream(socket));
    }

    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "could not resolve to any addresses",
    ));
  }

  pub fn send(
    &self,
    vec: Vec<u8>,
  ) -> impl Future<Output = lio::BufResult<i32, Vec<u8>>> {
    self.0.send(vec)
  }

  pub fn recv(
    &self,
    vec: Vec<u8>,
  ) -> impl Future<Output = lio::BufResult<i32, Vec<u8>>> {
    self.0.recv(vec)
  }
}

pub struct Socket(Fd);

impl Socket {
  pub async fn new(
    domain: Domain,
    ty: Type,
    proto: Option<Protocol>,
  ) -> io::Result<Self> {
    let rawfd = lio::socket(domain, ty, proto).await?;
    // SAFETY: We literally just created it.
    let fd = unsafe { Fd::from_raw_fd(rawfd) };
    return Ok(Socket(fd));
  }

  pub async fn bind(&self, addr: SocketAddr) -> io::Result<()> {
    lio::bind(self.0.0, addr).await
  }

  pub async fn listen(&self) -> io::Result<()> {
    lio::listen(self.0.0, 128).await
  }

  pub async fn accept(&self) -> io::Result<(Socket, SocketAddr)> {
    let (raw_fd, addr) = lio::accept(self.0.0).await?;
    let fd = unsafe { Fd::from_raw_fd(raw_fd) };

    Ok((Socket::from(fd), addr))
  }

  pub async fn connect(&self, addr: SocketAddr) -> io::Result<()> {
    lio::connect(self.0.0, addr).await
  }

  pub async fn recv(&self, vec: Vec<u8>) -> lio::BufResult<i32, Vec<u8>> {
    lio::recv(self.0.0, vec, None).await
  }

  pub async fn send(&self, vec: Vec<u8>) -> lio::BufResult<i32, Vec<u8>> {
    lio::send(self.0.0, vec, None).await
  }
}

impl From<Fd> for Socket {
  fn from(value: Fd) -> Self {
    Socket(value)
  }
}

// pub fn close(self) -> impl Future<Output = io::Result<()>> {
//   lio::close(self.0)
// }
//
// pub fn pwrite(
//   &self,
//   buf: Vec<u8>,
//   offset: i64,
// ) -> impl Future<Output = lio::BufResult<i32, Vec<u8>>> {
//   lio::write(self.0, buf, offset)
// }
//
// pub fn write(
//   &self,
//   buf: Vec<u8>,
// ) -> impl Future<Output = lio::BufResult<i32, Vec<u8>>> {
//   self.pwrite(buf, -1)
// }
//
// pub fn pread(
//   &self,
//   buf: Vec<u8>,
//   offset: i64,
// ) -> impl Future<Output = lio::BufResult<i32, Vec<u8>>> {
//   lio::read(self.0, buf, offset)
// }
//
// pub fn read(
//   &self,
//   buf: Vec<u8>,
// ) -> impl Future<Output = lio::BufResult<i32, Vec<u8>>> {
//   self.pread(buf, -1)
// }

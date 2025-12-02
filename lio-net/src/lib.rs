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
    for value in addr.to_socket_addrs()? {
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

    Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "could not resolve to any addresses",
    ))
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
    for value in addr.to_socket_addrs()? {
      let domain = match value {
        SocketAddr::V6(_) => Domain::IPV6,
        SocketAddr::V4(_) => Domain::IPV4,
      };
      let socket = Socket::new(domain, Type::STREAM, None).await?;

      socket.connect(value).await?;

      return Ok(TcpStream(socket));
    }

    Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "could not resolve to any addresses",
    ))
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

  pub fn shutdown(&self, how: i32) -> impl Future<Output = io::Result<()>> {
    self.0.shutdown(how)
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
    Ok(Socket(fd))
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
  pub async fn shutdown(&self, how: i32) -> io::Result<()> {
    lio::shutdown(self.0.0, how).await
  }
}

impl From<Fd> for Socket {
  fn from(value: Fd) -> Self {
    Socket(value)
  }
}

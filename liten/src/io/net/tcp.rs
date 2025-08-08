use std::{
  future::Future,
  io,
  net::{SocketAddr, ToSocketAddrs},
};

use socket2::{SockAddr, Type};

use crate::{
  future::{FutureExt, Stream},
  io::{net::socket::Socket, AsyncRead, AsyncWrite},
};

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
}

impl AsyncRead for TcpStream {
  fn read(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = crate::io::BufResult<usize, Vec<u8>>> {
    self.0.read(buf)
  }
}

impl AsyncWrite for TcpStream {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = crate::io::BufResult<usize, Vec<u8>>> {
    self.0.write(buf)
  }
  fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
    self.0.flush()
  }
}

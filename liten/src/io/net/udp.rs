use std::{
  future::Future,
  io,
  net::{SocketAddr, ToSocketAddrs},
};

use socket2::{SockAddr, Type};

use crate::{
  future::Stream,
  io::{net::socket::Socket, AsyncReadRent, AsyncWriteRent},
};

pub struct UdpSocket(Socket);

impl AsyncReadRent for UdpSocket {
  fn read(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = crate::io::BufResult<usize, Vec<u8>>> {
  }
}

impl UdpSocket {
  pub async fn bind<T: ToSocketAddrs>(addr: T) -> io::Result<Self> {
    let addr = SockAddr::from(addr.to_socket_addrs()?.next().unwrap());
    let socket = Socket::bind(addr, Type::DGRAM).await?;

    Ok(UdpSocket(socket))
  }
  pub async fn connect(addr: SocketAddr) -> io::Result<Self> {
    let socket = Socket::connect(addr.into(), Type::DGRAM).await?;
    Ok(Self(socket))
  }
}

impl AsyncReadRent for UdpSocket {
  fn read(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = crate::io::BufResult<usize, Vec<u8>>> {
    self.0.read(buf)
  }
}

impl AsyncWriteRent for UdpSocket {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = crate::io::BufResult<usize, Vec<u8>>> {
    self.0.write(buf)
  }
  fn flush(&mut self) -> io::Result<()> {
    self.0.flush()
  }
}

use super::{TcpListener, TcpStream};
use std::io;

pub struct Http1Listener {
  tcp: TcpListener,
}

pub struct Http1Request {
  stream: TcpStream,
  readable: bool,
}

impl Http1Request {
  pub fn from_stream(tcp: TcpStream) -> Self {
    Http1Request { stream: tcp, readable: false }
  }
}

impl Http1Listener {
  pub fn from_tcp(tcp: TcpListener) -> Self {
    Http1Listener { tcp }
  }

  pub async fn accept(&self) -> io::Result<Http1Request> {
    let (tcp_stream, _) = self.tcp.accept().await?;

    Ok(Http1Request::from_stream(tcp_stream))
  }
}

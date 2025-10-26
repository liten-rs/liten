use std::{mem::MaybeUninit, time::Duration};

use socket2::SockAddr;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  let socket = lio::socket(socket2::Domain::IPV4, socket2::Type::STREAM, None)
    .await
    .unwrap();
  let addr =
    SockAddr::from("127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap());
  lio::bind(socket, addr).await.unwrap();
  lio::listen(socket, 128).await.unwrap();

  let mut addr_storage: MaybeUninit<socket2::SockAddrStorage> =
    MaybeUninit::uninit();
  let mut addr_len =
    std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

  tokio::task::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
      interval.tick().await;
      println!("test");
    }
  });

  let client_fd =
    lio::accept(socket, &mut addr_storage, &mut addr_len).await.unwrap();

  let (res, _buf) = lio::recv(client_fd, vec![0, 0, 0], None).await;
  res.unwrap();

  lio::close(client_fd).await.unwrap();

  lio::close(socket).await.unwrap();
}

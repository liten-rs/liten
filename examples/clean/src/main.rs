use std::{
  ffi::c_void,
  io::Error,
  mem,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use liten::runtime::Runtime;
use socket2::SockAddr;

fn main() {
  Runtime::single_threaded().block_on(async {
    let tesing = liten::io::net::socket::Socket::new(
      socket2::Domain::IPV4,
      socket2::Type::STREAM,
    )
    .await
    .unwrap();

    let addr: SocketAddr = "[::1]:12345".parse().unwrap();
    tesing.bind(addr.into()).await.unwrap();
    tesing.listen().await.unwrap();

    let (fd, sock) = tesing.accept().await.unwrap();

    dbg!(fd, sock);

    // let result =
    //   tesing.connect(SockAddr::unix("/tmp/testing").unwrap()).await.unwrap();
    //
    // dbg!(
    //   unsafe {
    //     libc::send(result, Vec::from([1, 2]).as_ptr() as *const c_void, 2, 0)
    //   },
    //   Error::last_os_error()
    // );
  })
}

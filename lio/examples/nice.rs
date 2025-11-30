#![cfg(feature = "tracing")]

use std::{mem::MaybeUninit, net::SocketAddr, time::Duration};

use lio::{accept, bind, listen, socket};
use socket2::{Domain, Type};
use tracing::Level;

fn main() {
  tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
  liten::block_on(async {
    // Create and setup server socket
    let server_sock = socket(Domain::IPV4, Type::STREAM, None)
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind");

    // Get the bound address
    let bound_addr = unsafe {
      let mut addr_storage = MaybeUninit::<libc::sockaddr_in>::zeroed();
      let mut addr_len =
        std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
      libc::getsockname(
        server_sock,
        addr_storage.as_mut_ptr() as *mut libc::sockaddr,
        &mut addr_len,
      );
      let sockaddr_in = addr_storage.assume_init();
      let port = u16::from_be(sockaddr_in.sin_port);
      format!("127.0.0.1:{}", port).parse::<SocketAddr>().unwrap()
    };

    listen(server_sock, 128).await.expect("Failed to listen");

    let (accepted_fd, _) = accept(server_sock).await.unwrap();

    // // Spawn accept task
    // let accept_fut = async move {
    //   let (client_fd, _client_addr) =
    //     accept(server_sock).await.expect("Failed to accept");
    //
    //   (client_fd, server_sock)
    // };
    //
    // // Give accept time to start
    // let client_fut = async {
    //   // Connect client
    //   let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
    //     .await
    //     .expect("Failed to create client socket");
    //
    //   connect(client_sock, bound_addr).await.expect("Failed to connect");
    //
    //   client_sock
    // };

    // // Wait for accept
    // let (client_sock, (accepted_fd, server_sock)) =
    //   liten::join!(client_fut, accept_fut);
    //
    // assert!(accepted_fd >= 0, "Accepted fd should be valid");

    // Cleanup
    unsafe {
      // libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  });
}

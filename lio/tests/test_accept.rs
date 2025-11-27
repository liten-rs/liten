use lio::{accept, bind, connect, listen, socket};
use socket2::{Domain, Protocol, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;

#[test]
fn test_accept_basic() {
  liten::block_on(async {
    // Create and setup server socket
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
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

    // Spawn accept task
    let accept_fut = async move {
      let (client_fd, _client_addr) = accept(server_sock).await.expect("Failed to accept");

      (client_fd, server_sock)
    };

    // Give accept time to start
    let client_fut = async {
      // Connect client
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");

      client_sock
    };

    // Wait for accept
    let (client_sock, (accepted_fd, server_sock)) =
      liten::join!(client_fut, accept_fut);

    assert!(accepted_fd >= 0, "Accepted fd should be valid");

    // Cleanup
    unsafe {
      libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_accept_multiple() {
  liten::block_on(async {
    // Create server socket
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind");

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

    let num_clients = 5;
    let mut accepted_fds = Vec::new();
    let mut client_fds = Vec::new();

    for _ in 0..num_clients {
      // Spawn accept task
      let accept_fut = async move {
        let (client_fd, _client_addr) = accept(server_sock).await.expect("Failed to accept");

        (client_fd, server_sock)
      };

      let client_fut = async {
        // Connect client
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");

        connect(client_sock, bound_addr)
          .await
          .expect("Failed to connect");

        client_sock
      };

      let ((accepted_fd, _server_sock_returned), client_sock) =
        liten::join!(accept_fut, client_fut);
      accepted_fds.push(accepted_fd);
      client_fds.push(client_sock);
    }

    // Verify all connections
    assert_eq!(accepted_fds.len(), num_clients);
    assert_eq!(client_fds.len(), num_clients);

    // Cleanup
    unsafe {
      for fd in accepted_fds {
        libc::close(fd);
      }
      for fd in client_fds {
        libc::close(fd);
      }
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_accept_with_client_info() {
  liten::block_on(async {
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind");

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

    let accept_fut = async move {
      let (client_fd, _client_addr) = accept(server_sock).await.expect("Failed to accept");

      (client_fd, server_sock)
    };

    let client_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");

      client_sock
    };

    let (client_sock, (accepted_fd, server_sock)) =
      liten::join!(client_fut, accept_fut);

    // Cleanup
    unsafe {
      libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_accept_ipv6() {
  liten::block_on(async {
    let server_sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create IPv6 server socket");

    let addr: SocketAddr = "[::1]:0".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind IPv6");

    let bound_addr = unsafe {
      let mut addr_storage = MaybeUninit::<libc::sockaddr_in6>::zeroed();
      let mut addr_len =
        std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t;
      libc::getsockname(
        server_sock,
        addr_storage.as_mut_ptr() as *mut libc::sockaddr,
        &mut addr_len,
      );
      let sockaddr_in6 = addr_storage.assume_init();
      let port = u16::from_be(sockaddr_in6.sin6_port);
      format!("[::1]:{}", port).parse::<SocketAddr>().unwrap()
    };

    listen(server_sock, 128).await.expect("Failed to listen");

    let accept_fut = async move {
      let (client_fd, _client_addr) = accept(server_sock).await.expect("Failed to accept IPv6");

      (client_fd, server_sock)
    };

    let client_fut = async {
      let client_sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create IPv6 client socket");

      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect IPv6");

      client_sock
    };

    let (client_sock, (accepted_fd, server_sock)) =
      liten::join!(client_fut, accept_fut);

    assert!(accepted_fd >= 0);

    // Cleanup
    unsafe {
      libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_accept_concurrent() {
  liten::block_on(async {
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind");

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

    // Run accepts and connects concurrently
    let accept_fut = async {
      let mut accepted_fds = Vec::new();
      for _ in 0..3 {
        let (client_fd, _client_addr) = accept(server_sock).await.expect("Failed to accept");

        accepted_fds.push(client_fd);
      }
      (accepted_fds, server_sock)
    };

    let connect_fut = async {
      let mut client_fds = Vec::new();
      for _ in 0..3 {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");

        connect(client_sock, bound_addr)
          .await
          .expect("Failed to connect");

        client_fds.push(client_sock);
      }
      client_fds
    };

    let ((accepted_fds, server_sock), client_fds) =
      liten::join!(accept_fut, connect_fut);

    assert_eq!(accepted_fds.len(), 3);

    // Cleanup
    unsafe {
      for fd in accepted_fds {
        libc::close(fd);
      }
      for fd in client_fds {
        libc::close(fd);
      }
      libc::close(server_sock);
    }
  });
}

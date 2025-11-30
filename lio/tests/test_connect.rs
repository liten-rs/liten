#![cfg(feature = "high")]
use lio::{bind, connect, listen, socket};
use socket2::{Domain, Protocol, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use tracing::Level;

#[test]
fn test_connect_basic() {
  liten::block_on(async {
    // Create server socket
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    println!("bind");
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

    println!("listen");
    listen(server_sock, 128).await.expect("Failed to listen");

    println!("socket");
    // Create client socket and connect
    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    println!("connect");
    connect(client_sock, bound_addr).await.expect("Failed to connect");
    println!("connect after");

    // Verify connection by checking peer name
    unsafe {
      let mut peer_addr = MaybeUninit::<libc::sockaddr_storage>::zeroed();
      let mut peer_len =
        std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
      let result = libc::getpeername(
        client_sock,
        peer_addr.as_mut_ptr() as *mut libc::sockaddr,
        &mut peer_len,
      );
      assert_eq!(result, 0, "Should be able to get peer name after connect");

      libc::close(client_sock);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_connect_ipv6() {
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

    let client_sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create IPv6 client socket");

    connect(client_sock, bound_addr).await.expect("Failed to connect IPv6");

    unsafe {
      let mut peer_addr = MaybeUninit::<libc::sockaddr_storage>::zeroed();
      let mut peer_len =
        std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
      let result = libc::getpeername(
        client_sock,
        peer_addr.as_mut_ptr() as *mut libc::sockaddr,
        &mut peer_len,
      );
      assert_eq!(result, 0);

      libc::close(client_sock);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_connect_to_nonexistent() {
  liten::block_on(async {
    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    // Try to connect to a port that's (hopefully) not listening
    let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();

    let result = connect(client_sock, addr).await;

    // Should fail with connection refused
    assert!(result.is_err(), "Connect to non-listening port should fail");

    unsafe {
      libc::close(client_sock);
    }
  });
}

#[test]
fn test_connect_multiple_clients() {
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

    // Connect multiple clients
    let mut client_socks = Vec::new();
    for _ in 0..5 {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      connect(client_sock, bound_addr).await.expect("Failed to connect");

      client_socks.push(client_sock);
    }

    assert_eq!(client_socks.len(), 5);

    // Cleanup
    unsafe {
      for sock in client_socks {
        libc::close(sock);
      }
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_connect_already_connected() {
  tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
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

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    connect(client_sock, bound_addr)
      .await
      .expect("First connect should succeed");

    // Try to connect again
    let result = connect(client_sock, bound_addr).await;

    // Should fail with already connected
    assert!(result.is_err(), "Second connect should fail: err {result:#?}");

    unsafe {
      libc::close(client_sock);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_connect_to_localhost() {
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

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    connect(client_sock, bound_addr)
      .await
      .expect("Failed to connect to localhost");

    // Verify connected to localhost
    unsafe {
      let mut peer_addr = MaybeUninit::<libc::sockaddr_in>::zeroed();
      let mut peer_len =
        std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
      libc::getpeername(
        client_sock,
        peer_addr.as_mut_ptr() as *mut libc::sockaddr,
        &mut peer_len,
      );
      let sockaddr_in = peer_addr.assume_init();
      assert_eq!(u32::from_be(sockaddr_in.sin_addr.s_addr), 0x7f000001);

      libc::close(client_sock);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_connect_concurrent() {
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

    // Connect multiple clients sequentially
    let mut client_socks = Vec::new();
    for _ in 0..10 {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      connect(client_sock, bound_addr).await.expect("Failed to connect");

      client_socks.push(client_sock);
    }

    assert_eq!(client_socks.len(), 10);

    // Cleanup
    unsafe {
      for sock in client_socks {
        libc::close(sock);
      }
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_connect_with_bind() {
  liten::block_on(async {
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind server");

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

    // Create client socket and bind it to a specific local address
    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    let client_bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    bind(client_sock, client_bind_addr).await.expect("Failed to bind client");

    // Now connect
    connect(client_sock, bound_addr).await.expect("Failed to connect");

    unsafe {
      libc::close(client_sock);
      libc::close(server_sock);
    }
  });
}

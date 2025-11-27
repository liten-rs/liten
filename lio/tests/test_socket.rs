use lio::socket;
use socket2::{Domain, Protocol, Type};

#[test]
fn test_socket_simple() {
  liten::block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create TCP IPv4 socket");

    assert!(sock >= 0, "Socket fd should be valid");

    // Verify it's a TCP socket
    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_STREAM);
      lio::close(sock).await.unwrap();
    }
  });
}

#[test]
fn test_socket_tcp_ipv4() {
  liten::block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create TCP IPv4 socket");

    assert!(sock >= 0, "Socket fd should be valid");

    // Verify it's a TCP socket
    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_STREAM);
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_tcp_ipv6() {
  liten::block_on(async {
    let sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create TCP IPv6 socket");

    assert!(sock >= 0, "Socket fd should be valid");

    // Verify it's a TCP socket
    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_STREAM);
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_udp_ipv4() {
  liten::block_on(async {
    let sock = socket(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
      .await
      .expect("Failed to create UDP IPv4 socket");

    assert!(sock >= 0, "Socket fd should be valid");

    // Verify it's a UDP socket
    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_DGRAM);
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_udp_ipv6() {
  liten::block_on(async {
    let sock = socket(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
      .await
      .expect("Failed to create UDP IPv6 socket");

    assert!(sock >= 0, "Socket fd should be valid");

    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_DGRAM);
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_without_protocol() {
  liten::block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, None)
      .await
      .expect("Failed to create socket without explicit protocol");

    assert!(sock >= 0, "Socket fd should be valid");

    unsafe {
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_unix_stream() {
  liten::block_on(async {
    let sock = socket(Domain::UNIX, Type::STREAM, None)
      .await
      .expect("Failed to create Unix stream socket");

    assert!(sock >= 0, "Socket fd should be valid");

    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_STREAM);
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_unix_dgram() {
  liten::block_on(async {
    let sock = socket(Domain::UNIX, Type::DGRAM, None)
      .await
      .expect("Failed to create Unix datagram socket");

    assert!(sock >= 0, "Socket fd should be valid");

    unsafe {
      let mut sock_type: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut sock_type as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(sock_type, libc::SOCK_DGRAM);
      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_multiple() {
  liten::block_on(async {
    // Create multiple sockets
    let sock1 = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create first socket");
    let sock2 = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create second socket");
    let sock3 = socket(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
      .await
      .expect("Failed to create third socket");

    assert!(sock1 >= 0);
    assert!(sock2 >= 0);
    assert!(sock3 >= 0);
    assert_ne!(sock1, sock2);
    assert_ne!(sock1, sock3);
    assert_ne!(sock2, sock3);

    // Cleanup
    unsafe {
      libc::close(sock1);
      libc::close(sock2);
      libc::close(sock3);
    }
  });
}

#[test]
fn test_socket_concurrent() {
  liten::block_on(async {
    // Test creating multiple sockets sequentially
    for i in 0..20 {
      let domain = if i % 2 == 0 { Domain::IPV4 } else { Domain::IPV6 };
      let ty = if i % 3 == 0 { Type::DGRAM } else { Type::STREAM };
      let proto = if ty == Type::STREAM {
        Some(Protocol::TCP)
      } else {
        Some(Protocol::UDP)
      };

      let sock =
        socket(domain, ty, proto).await.expect("Failed to create socket");

      assert!(sock >= 0);

      unsafe {
        libc::close(sock);
      }
    }
  });
}

#[test]
fn test_socket_options_after_creation() {
  liten::block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    // Test setting socket options
    unsafe {
      let reuse_val: i32 = 1;
      let result = libc::setsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_REUSEADDR,
        &reuse_val as *const _ as *const libc::c_void,
        std::mem::size_of::<i32>() as libc::socklen_t,
      );
      assert_eq!(result, 0, "Failed to set SO_REUSEADDR");

      // Verify the option was set
      let mut get_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_REUSEADDR,
        &mut get_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_ne!(get_val, 0);

      libc::close(sock);
    }
  });
}

#[test]
fn test_socket_nonblocking() {
  liten::block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    // Set non-blocking mode
    unsafe {
      let flags = libc::fcntl(sock, libc::F_GETFL, 0);
      libc::fcntl(sock, libc::F_SETFL, flags | libc::O_NONBLOCK);

      // Verify non-blocking mode is set
      let new_flags = libc::fcntl(sock, libc::F_GETFL, 0);
      assert!(new_flags & libc::O_NONBLOCK != 0);

      libc::close(sock);
    }
  });
}

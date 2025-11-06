use lio::loom::test_utils::{block_on, model};
use lio::{bind, socket};
use socket2::{Domain, Protocol, SockAddr, Type};
use std::net::SocketAddr;

#[test]
fn test_bind_ipv4_any_port() {
  model(|| {
    block_on(async {
      let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create socket");

      // Bind to 0.0.0.0:0 (any available port)
      let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock, sock_addr).await.expect("Failed to bind socket");

      // Verify binding by getting the socket name
      unsafe {
        let mut addr_storage =
          std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();
        let mut addr_len =
          std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let result = libc::getsockname(
          sock,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        assert_eq!(result, 0, "getsockname should succeed");
        libc::close(sock);
      }
    })
  })
}

#[test]
fn test_bind_ipv4_specific_port() {
  model(|| {
    block_on(async {
      let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create socket");

      // Enable SO_REUSEADDR
      unsafe {
        let reuse: i32 = 1;
        libc::setsockopt(
          sock,
          libc::SOL_SOCKET,
          libc::SO_REUSEADDR,
          &reuse as *const _ as *const libc::c_void,
          std::mem::size_of::<i32>() as libc::socklen_t,
        );
      }

      // Bind to a high port number
      let addr: SocketAddr = "127.0.0.1:19999".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock, sock_addr).await.expect("Failed to bind to specific port");

      // Verify the port
      unsafe {
        let mut addr_storage =
          std::mem::MaybeUninit::<libc::sockaddr_in>::zeroed();
        let mut addr_len =
          std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        libc::getsockname(
          sock,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        let sockaddr_in = addr_storage.assume_init();
        assert_eq!(u16::from_be(sockaddr_in.sin_port), 19999);
        libc::close(sock);
      }
    })
  })
}

#[test]
fn test_bind_ipv6() {
  model(|| {
    block_on(async {
      let sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create IPv6 socket");

      // Bind to IPv6 any address
      let addr: SocketAddr = "[::]:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock, sock_addr).await.expect("Failed to bind IPv6 socket");

      // Verify binding
      unsafe {
        let mut addr_storage =
          std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();
        let mut addr_len =
          std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let result = libc::getsockname(
          sock,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        assert_eq!(result, 0);
        libc::close(sock);
      }
    })
  })
}

#[test]
fn test_bind_udp() {
  model(|| {
    block_on(async {
      let sock = socket(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
        .await
        .expect("Failed to create UDP socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock, sock_addr).await.expect("Failed to bind UDP socket");

      // Verify binding
      unsafe {
        let mut addr_storage =
          std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();
        let mut addr_len =
          std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let result = libc::getsockname(
          sock,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        assert_eq!(result, 0);
        libc::close(sock);
      }
    })
  })
}

#[test]
fn test_bind_already_bound() {
  model(|| {
    block_on(async {
      let sock1 = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create first socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock1, sock_addr).await.expect("Failed to bind first socket");

      // Get the actual bound address
      let bound_addr = unsafe {
        let mut addr_storage =
          std::mem::MaybeUninit::<libc::sockaddr_in>::zeroed();
        let mut addr_len =
          std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        libc::getsockname(
          sock1,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        addr_storage.assume_init()
      };

      // Try to bind another socket to the same address
      let sock2 = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create second socket");

      let port = u16::from_be(bound_addr.sin_port);
      let addr2: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
      let sock_addr2 = SockAddr::from(addr2);

      let result = bind(sock2, sock_addr2).await;

      // Should fail with address in use
      assert!(result.is_err(), "Binding to already-used address should fail");

      // Cleanup
      unsafe {
        libc::close(sock1);
        libc::close(sock2);
      }
    })
  })
}

#[test]
fn test_bind_double_bind() {
  model(|| {
    block_on(async {
      let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock, sock_addr.clone()).await.expect("First bind should succeed");

      // Try to bind the same socket again
      let result = bind(sock, sock_addr).await;

      // Should fail
      assert!(result.is_err(), "Double bind should fail");

      // Cleanup
      unsafe {
        libc::close(sock);
      }
    })
  })
}

#[test]
fn test_bind_with_reuseaddr() {
  model(|| {
    block_on(async {
      let sock1 = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create first socket");

      // Enable SO_REUSEADDR on first socket
      unsafe {
        let reuse: i32 = 1;
        libc::setsockopt(
          sock1,
          libc::SOL_SOCKET,
          libc::SO_REUSEADDR,
          &reuse as *const _ as *const libc::c_void,
          std::mem::size_of::<i32>() as libc::socklen_t,
        );
      }

      let addr: SocketAddr = "127.0.0.1:18888".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock1, sock_addr.clone())
        .await
        .expect("Failed to bind first socket");

      // Close first socket
      unsafe {
        libc::close(sock1);
      }

      // Immediately bind another socket to the same address with SO_REUSEADDR
      let sock2 = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create second socket");

      unsafe {
        let reuse: i32 = 1;
        libc::setsockopt(
          sock2,
          libc::SOL_SOCKET,
          libc::SO_REUSEADDR,
          &reuse as *const _ as *const libc::c_void,
          std::mem::size_of::<i32>() as libc::socklen_t,
        );
      }

      bind(sock2, sock_addr)
        .await
        .expect("Should be able to bind with SO_REUSEADDR after closing previous socket");

      // Cleanup
      unsafe {
        libc::close(sock2);
      }
    })
  })
}

#[test]
fn test_bind_localhost() {
  model(|| {
    block_on(async {
      let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);

      bind(sock, sock_addr).await.expect("Failed to bind to localhost");

      // Verify it's bound to localhost
      unsafe {
        let mut addr_storage =
          std::mem::MaybeUninit::<libc::sockaddr_in>::zeroed();
        let mut addr_len =
          std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        libc::getsockname(
          sock,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        let sockaddr_in = addr_storage.assume_init();
        // 127.0.0.1 in network byte order
        assert_eq!(u32::from_be(sockaddr_in.sin_addr.s_addr), 0x7f000001);
        libc::close(sock);
      }
    })
  })
}

#[test]
fn test_bind_concurrent() {
  model(|| {
    block_on(async {
      // Test binding multiple sockets concurrently to different ports
      let tasks: Vec<_> = (20000..20010)
        .map(|port| async move {
          let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create socket");

          unsafe {
            let reuse: i32 = 1;
            libc::setsockopt(
              sock,
              libc::SOL_SOCKET,
              libc::SO_REUSEADDR,
              &reuse as *const _ as *const libc::c_void,
              std::mem::size_of::<i32>() as libc::socklen_t,
            );
          }

          let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
          let sock_addr = SockAddr::from(addr);

          bind(sock, sock_addr).await.expect("Failed to bind socket");

          unsafe {
            libc::close(sock);
          }
        })
        .collect();

      for task in tasks {
        task.await;
      }
    })
  })
}

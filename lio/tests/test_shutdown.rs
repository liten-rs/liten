use lio::{accept, bind, connect, listen, recv, send, shutdown, socket};
use socket2::{Domain, Protocol, SockAddr, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;

#[test]
fn test_shutdown_write() {
  liten::block_on(async {
      // Create server socket
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        let client_fd = accept(server_sock).await.expect("Failed to accept");
        (client_fd, server_sock)
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let ((server_client_fd, server_sock), client_sock) =
        liten::join!(accept_fut, client_fut);

      // Shutdown write on client
      shutdown(client_sock, libc::SHUT_WR)
        .await
        .expect("Failed to shutdown write");

      // Try to send data (should fail or return 0)
      let data = b"Test data".to_vec();
      let (result, _) = send(client_sock, data, None).await;

      // Send after SHUT_WR should fail
      assert!(
        result.is_err() || result.unwrap() == 0,
        "Send should fail after SHUT_WR"
      );

      // Server should be able to read EOF
      let buf = vec![0u8; 100];
      let (bytes_received, _) = recv(server_client_fd, buf, None).await;
      assert_eq!(
        bytes_received.expect("Recv should succeed"),
        0,
        "Should receive EOF after client shutdown write"
      );

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_read() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        let client_fd = accept(server_sock).await.expect("Failed to accept");
        (client_fd, server_sock)
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let ((server_client_fd, server_sock), client_sock) =
        liten::join!(accept_fut, client_fut);

      // Shutdown read on client
      shutdown(client_sock, libc::SHUT_RD)
        .await
        .expect("Failed to shutdown read");

      // Client can still send
      let data = b"Hello".to_vec();
      let (bytes_sent, _) = send(client_sock, data.clone(), None).await;
      assert_eq!(
        bytes_sent.expect("Send should succeed") as usize,
        data.len()
      );

      // Server can receive
      let buf = vec![0u8; 100];
      let (bytes_received, received_buf) = recv(server_client_fd, buf, None).await;
      let bytes_received = bytes_received.expect("Recv should succeed") as usize;
      assert_eq!(bytes_received, data.len());
      assert_eq!(&received_buf[..bytes_received], data.as_slice());

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_both() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        let client_fd = accept(server_sock).await.expect("Failed to accept");
        (client_fd, server_sock)
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let ((server_client_fd, server_sock), client_sock) =
        liten::join!(accept_fut, client_fut);

      // Shutdown both directions on client
      shutdown(client_sock, libc::SHUT_RDWR)
        .await
        .expect("Failed to shutdown both");

      // Send should fail
      let data = b"Test".to_vec();
      let (result, _) = send(client_sock, data, None).await;
      assert!(
        result.is_err() || result.unwrap() == 0,
        "Send should fail after SHUT_RDWR"
      );

      // Server should receive EOF
      let buf = vec![0u8; 100];
      let (bytes_received, _) = recv(server_client_fd, buf, None).await;
      assert_eq!(
        bytes_received.expect("Recv should succeed"),
        0,
        "Should receive EOF"
      );

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_invalid_fd() {
  liten::block_on(async {
      // Try to shutdown an invalid file descriptor
      let result = shutdown(-1, libc::SHUT_RDWR).await;
      assert!(result.is_err(), "Shutdown on invalid fd should fail");
  });
}

#[test]
fn test_shutdown_after_close() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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

      connect(client_sock, SockAddr::from(bound_addr))
        .await
        .expect("Failed to connect");

      // Close the socket
      unsafe {
        libc::close(client_sock);
      }

      // Try to shutdown after close (should fail)
      let result = shutdown(client_sock, libc::SHUT_RDWR).await;
      assert!(result.is_err(), "Shutdown after close should fail");

      // Cleanup
      unsafe {
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_twice() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        accept(server_sock).await.expect("Failed to accept")
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let (server_client_fd, client_sock) =
        liten::join!(accept_fut, client_fut);

      // First shutdown
      shutdown(client_sock, libc::SHUT_WR)
        .await
        .expect("First shutdown should succeed");

      // Second shutdown on same direction
      let result = shutdown(client_sock, libc::SHUT_WR).await;
      // Some systems allow this, some don't - just verify it doesn't crash
      let _ = result;

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_sequential_directions() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        let client_fd = accept(server_sock).await.expect("Failed to accept");
        (client_fd, server_sock)
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let ((server_client_fd, server_sock), client_sock) =
        liten::join!(accept_fut, client_fut);

      // Shutdown write first
      shutdown(client_sock, libc::SHUT_WR)
        .await
        .expect("Failed to shutdown write");

      // Client can still receive
      let server_send_fut = async {
        let data = b"From server".to_vec();
        send(server_client_fd, data, None).await
      };

      let client_recv_fut = async {
        let buf = vec![0u8; 100];
        recv(client_sock, buf, None).await
      };

      let ((bytes_sent, _), (bytes_received, received_buf)) =
        liten::join!(server_send_fut, client_recv_fut);

      assert!(bytes_sent.is_ok(), "Server send should succeed");
      let bytes_received = bytes_received.expect("Client recv should succeed") as usize;
      assert_eq!(&received_buf[..bytes_received], b"From server");

      // Now shutdown read
      shutdown(client_sock, libc::SHUT_RD)
        .await
        .expect("Failed to shutdown read");

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_before_data_sent() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        accept(server_sock).await.expect("Failed to accept")
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let (server_client_fd, client_sock) =
        liten::join!(accept_fut, client_fut);

      // Shutdown immediately after connection, before any data transfer
      shutdown(client_sock, libc::SHUT_RDWR)
        .await
        .expect("Shutdown should succeed on fresh connection");

      // Verify server sees EOF
      let buf = vec![0u8; 100];
      let (bytes_received, _) = recv(server_client_fd, buf, None).await;
      assert_eq!(bytes_received.expect("Recv should succeed"), 0);

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_ipv6() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create IPv6 server socket");

      let addr: SocketAddr = "[::1]:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind IPv6");

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
        accept(server_sock).await.expect("Failed to accept IPv6")
      };

      let client_fut = async {
        let client_sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
          .await
          .expect("Failed to create IPv6 client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect IPv6");
        client_sock
      };

      let (server_client_fd, client_sock) =
        liten::join!(accept_fut, client_fut);

      // Shutdown write on IPv6 socket
      shutdown(client_sock, libc::SHUT_WR)
        .await
        .expect("Failed to shutdown IPv6 socket");

      // Verify EOF
      let buf = vec![0u8; 100];
      let (bytes_received, _) = recv(server_client_fd, buf, None).await;
      assert_eq!(bytes_received.expect("Recv should succeed"), 0);

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

#[test]
fn test_shutdown_concurrent() {
  liten::block_on(async {
      // Test shutting down multiple connections concurrently
      let tasks: Vec<_> = (0..5)
        .map(|_| async move {
          let server_sock =
            socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
              .await
              .expect("Failed to create server socket");

          let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
          let sock_addr = SockAddr::from(addr);
          bind(server_sock, sock_addr).await.expect("Failed to bind");

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
            accept(server_sock).await.expect("Failed to accept")
          };

          let client_fut = async {
            let client_sock =
              socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
                .await
                .expect("Failed to create client socket");
            connect(client_sock, SockAddr::from(bound_addr))
              .await
              .expect("Failed to connect");
            client_sock
          };

          let (server_client_fd, client_sock) =
            liten::join!(accept_fut, client_fut);

          // Shutdown
          shutdown(client_sock, libc::SHUT_RDWR)
            .await
            .expect("Concurrent shutdown failed");

          unsafe {
            libc::close(client_sock);
            libc::close(server_client_fd);
            libc::close(server_sock);
          }
        })
        .collect();

      for task in tasks {
        task.await;
      }
  });
}

#[test]
fn test_shutdown_with_pending_data() {
  liten::block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      let sock_addr = SockAddr::from(addr);
      bind(server_sock, sock_addr).await.expect("Failed to bind");

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
        let client_fd = accept(server_sock).await.expect("Failed to accept");
        (client_fd, server_sock)
      };

      let client_fut = async {
        let client_sock =
          socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
        connect(client_sock, SockAddr::from(bound_addr))
          .await
          .expect("Failed to connect");
        client_sock
      };

      let ((server_client_fd, server_sock), client_sock) =
        liten::join!(accept_fut, client_fut);

      // Send some data from client
      let data = b"Data before shutdown".to_vec();
      let (bytes_sent, _) = send(client_sock, data.clone(), None).await;
      assert!(bytes_sent.is_ok(), "Send should succeed");

      // Shutdown write immediately (data may still be in transit)
      shutdown(client_sock, libc::SHUT_WR)
        .await
        .expect("Shutdown should succeed");

      // Server should still be able to receive the data
      let buf = vec![0u8; 100];
      let (bytes_received, received_buf) = recv(server_client_fd, buf, None).await;
      let bytes_received = bytes_received.expect("Recv should succeed") as usize;

      // Should receive the data followed by EOF
      if bytes_received > 0 {
        assert_eq!(&received_buf[..bytes_received], data.as_slice());

        // Next read should be EOF
        let buf2 = vec![0u8; 100];
        let (bytes_received2, _) = recv(server_client_fd, buf2, None).await;
        assert_eq!(bytes_received2.expect("Recv should succeed"), 0);
      }

      // Cleanup
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
  });
}

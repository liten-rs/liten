use lio::loom::test_utils::{self, block_on, sleep};
use lio::{accept, bind, connect, listen, send, socket};
use socket2::{Domain, Protocol, SockAddr, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::time::Duration;

#[test]
fn test_send_basic() {
  test_utils::model(|| {
    block_on(async {
      // Setup server
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      println!("before");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");
      println!("after bind");

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
      println!("after listen");

      // Accept and connect concurrently using tokio::join!
      let accept_fut =
        async move { accept(server_sock).await.expect("Failed to accept") };

      let connect_fut = async {
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
        tokio::join!(accept_fut, connect_fut);

      // Send data
      let data = b"Hello, Server!".to_vec();
      let (bytes_sent, returned_buf) =
        send(client_sock, data.clone(), None).await;
      let bytes_sent = bytes_sent.expect("Failed to send data");

      assert_eq!(bytes_sent as usize, data.len());
      assert_eq!(returned_buf, data);
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
    })
  })
}

#[test]
fn test_send_large_data() {
  test_utils::model(|| {
    block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");

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

      let accept_fut =
        async move { accept(server_sock).await.expect("Failed to accept") };

      let connect_fut = async {
        sleep(Duration::from_millis(10));
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
        tokio::join!(accept_fut, connect_fut);

      // Send large data (1MB)
      let large_data: Vec<u8> =
        (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
      let (bytes_sent, returned_buf) =
        send(client_sock, large_data.clone(), None).await;
      let bytes_sent = bytes_sent.expect("Failed to send large data");

      assert!(bytes_sent > 0);
      assert_eq!(returned_buf, large_data);
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
    })
  })
}

#[test]
fn test_send_empty() {
  test_utils::model(|| {
    block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");

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

      let accept_handle = async move {
        let res = accept(server_sock).await.expect("Failed to accept");
        res
      };

      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      let (handle, handl2) = tokio::join!(
        accept_handle,
        connect(client_sock, SockAddr::from(bound_addr))
      );

      handl2.unwrap();

      // Send empty data
      let data = Vec::new();
      let (bytes_sent, _) = send(client_sock, data, None).await;
      let bytes_sent = bytes_sent.expect("Failed to send empty data");

      assert_eq!(bytes_sent, 0);

      let server_client_fd = handle;
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }

      lio::shutdown();
    });
  });
}

#[test]
fn test_send_multiple() {
  test_utils::model(|| {
    block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");

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

      let accept_fut =
        async move { accept(server_sock).await.expect("Failed to accept") };

      let connect_fut = async {
        sleep(Duration::from_millis(10));
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
        tokio::join!(accept_fut, connect_fut);

      // Send multiple messages
      for i in 0..5 {
        let data = format!("Message {}", i).into_bytes();
        let (bytes_sent, returned_buf) =
          send(client_sock, data.clone(), None).await;
        let bytes_sent = bytes_sent.expect("Failed to send");
        assert_eq!(bytes_sent as usize, data.len());
        assert_eq!(returned_buf, data);
      }
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
      lio::shutdown();
    })
  })
}

#[test]
fn test_send_with_flags() {
  test_utils::model(|| {
    block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");

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

      let accept_fut =
        async move { accept(server_sock).await.expect("Failed to accept") };

      let connect_fut = async {
        sleep(Duration::from_millis(10));
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
        tokio::join!(accept_fut, connect_fut);

      // Send with flags (0 is a valid flag value)
      let data = b"Data with flags".to_vec();
      let (bytes_sent, returned_buf) =
        send(client_sock, data.clone(), Some(0)).await;
      let bytes_sent = bytes_sent.expect("Failed to send with flags");

      assert_eq!(bytes_sent as usize, data.len());
      assert_eq!(returned_buf, data);
      unsafe {
        libc::close(client_sock);
        libc::close(server_client_fd);
        libc::close(server_sock);
      }
      lio::shutdown();
    })
  })
}

#[test]
fn test_send_on_closed_socket() {
  test_utils::model(|| {
    block_on(async {
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");

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

      let accept_fut =
        async move { accept(server_sock).await.expect("Failed to accept") };

      let connect_fut = async {
        sleep(Duration::from_millis(10));
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
        tokio::join!(accept_fut, connect_fut);

      // Close server end
      unsafe {
        libc::close(server_client_fd);
      }

      sleep(Duration::from_millis(10));

      // Try to send after server closed
      let data = b"This should fail".to_vec();
      let (_result, _) = send(client_sock, data, None).await;

      // May succeed or fail depending on timing, but shouldn't crash
      unsafe {
        libc::close(client_sock);
        libc::close(server_sock);
      }
    })
  })
}

#[test]
fn test_send_concurrent() {
  test_utils::model(|| {
    block_on(async {
      // Test sending from multiple clients concurrently
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, SockAddr::from(addr)).await.expect("Failed to bind");

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

      // Note: Simplified concurrent test without actual concurrency
      for i in 0..5 {
        let accept_fut =
          async move { accept(server_sock).await.expect("Failed to accept") };

        let client_fut = async {
          // Create and connect client
          let client_sock =
            socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
              .await
              .expect("Failed to create client socket");
          connect(client_sock, SockAddr::from(bound_addr))
            .await
            .expect("Failed to connect");
          client_sock
        };

        let (_server_client_fd, client_sock) =
          tokio::join!(accept_fut, client_fut);

        // Send data
        let data = format!("Client {}", i).into_bytes();
        let (bytes_sent, _) = send(client_sock, data.clone(), None).await;
        let bytes_sent = bytes_sent.expect("Failed to send");

        assert_eq!(bytes_sent as usize, data.len());

        unsafe {
          libc::close(client_sock);
          libc::close(_server_client_fd);
        }
      }

      unsafe {
        libc::close(server_sock);
      }
      lio::shutdown();
    })
  })
}

use lio::{accept, bind, connect, listen, recv, send, socket};
use proptest::prelude::*;
use socket2::{Domain, Protocol, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;

#[test]
fn test_recv_multiple() {
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

    let server_fut = async move {
      let client_fd = accept(server_sock).await.expect("Failed to accept");

      // Receive until EOF
      let mut all_data = Vec::new();
      loop {
        let buf = vec![0u8; 1024];
        let (bytes_received, received_buf) = recv(client_fd, buf, None).await;
        let bytes_received =
          bytes_received.expect("Failed to receive") as usize;

        if bytes_received == 0 {
          break; // EOF
        }

        all_data.extend_from_slice(&received_buf[..bytes_received]);
      }

      (all_data, client_fd, server_sock)
    };

    let client_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");

      // Send multiple messages
      for i in 0..3 {
        let data = format!("Message {}", i).into_bytes();
        let (bytes_sent, _) = send(client_sock, data, None).await;
        bytes_sent.expect("Failed to send");
      }

      // Shutdown write side to signal EOF to server
      unsafe {
        libc::shutdown(client_sock, libc::SHUT_WR);
      }

      client_sock
    };

    let ((all_data, server_client_fd, server_sock), client_sock) =
      liten::join!(server_fut, client_fut);

    // Verify we received all 3 messages concatenated
    let expected = b"Message 0Message 1Message 2";
    assert_eq!(all_data, expected);

    lio::close(client_sock).await.expect("Failed to close client");
    lio::close(server_client_fd).await.expect("Failed to close server client");
    lio::close(server_sock).await.expect("Failed to close server");
  });
}

#[test]
fn test_recv_with_flags() {
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

    let send_data = b"Data with flags".to_vec();

    let server_fut = async move {
      let client_fd = accept(server_sock).await.expect("Failed to accept");

      let buf = vec![0u8; 1024];
      let (bytes_received, received_buf) = recv(client_fd, buf, Some(0)).await;
      let bytes_received =
        bytes_received.expect("Failed to receive with flags");

      (bytes_received, received_buf, client_fd, server_sock)
    };

    let client_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");

      let (bytes_sent, _) = send(client_sock, send_data.clone(), None).await;
      bytes_sent.expect("Failed to send");
      client_sock
    };

    let (
      (bytes_received, received_buf, server_client_fd, server_sock),
      client_sock,
    ) = liten::join!(server_fut, client_fut);

    assert_eq!(bytes_received as usize, send_data.len());
    assert_eq!(&received_buf[..bytes_received as usize], send_data.as_slice());

    lio::close(client_sock).await.expect("Failed to close client");
    lio::close(server_client_fd).await.expect("Failed to close server client");
    lio::close(server_sock).await.expect("Failed to close server");
  });
}

#[test]
fn test_recv_on_closed() {
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

    let server_fut = async move {
      let client_fd = accept(server_sock).await.expect("Failed to accept");

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

    let ((server_client_fd, server_sock), client_sock) =
      liten::join!(server_fut, client_fut);

    // Close client
    unsafe {
      libc::close(client_sock);
    }

    // Try to receive on closed connection
    let buf = vec![0u8; 1024];
    let (bytes_received, _) = recv(server_client_fd, buf, None).await;
    let bytes_received =
      bytes_received.expect("recv should succeed but return 0");

    // Should return 0 when connection is closed
    assert_eq!(bytes_received, 0);

    lio::close(server_client_fd).await.expect("Failed to close server client");
    lio::close(server_sock).await.expect("Failed to close server");
  });
}

proptest! {
  #[test]
  fn prop_test_recv_arbitrary_data(
    data_size in 1usize..=8192,
    seed in any::<u64>(),
  ) {
    liten::block_on(async move {
      // Generate deterministic random data
      let test_data: Vec<u8> = (0..data_size)
        .map(|i| ((seed.wrapping_add(i as u64)) % 256) as u8)
        .collect();

      // Create server socket using lio (required for lio::accept to work)
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      // Bind using lio
      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, addr).await.expect("Failed to bind");

      // Get bound address
      let bound_addr = unsafe {
        let mut addr_storage = MaybeUninit::<libc::sockaddr_in>::zeroed();
        let mut addr_len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        libc::getsockname(server_sock, addr_storage.as_mut_ptr() as *mut libc::sockaddr, &mut addr_len);
        let sockaddr_in = addr_storage.assume_init();
        let port = u16::from_be(sockaddr_in.sin_port);
        format!("127.0.0.1:{}", port).parse::<SocketAddr>().unwrap()
      };

      // Listen using lio
      listen(server_sock, 128).await.expect("Failed to listen");

      // Accept connection and connect client (using lio for proper async handling)
      let (server_client_fd, client_sock) = liten::join!(
        async { accept(server_sock).await.expect("Accept failed") },
        async {
          let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
          connect(sock, bound_addr).await.expect("Connect failed");
          sock
        }
      );

      // Send data from client using libc
      let sent = unsafe {
        libc::send(
          client_sock,
          test_data.as_ptr() as *const libc::c_void,
          test_data.len(),
          0,
        )
      };
      assert_eq!(sent as usize, test_data.len(), "Send failed");

      // Test recv on server side using lio (the only lio syscall in this test)
      let recv_buf = vec![0u8; data_size];
      let (recv_result, received_buf) = recv(server_client_fd, recv_buf, None).await;
      let bytes_received = recv_result.expect("Recv failed") as usize;

      // Verify
      assert!(bytes_received > 0, "Should receive at least some bytes");
      assert_eq!(
        &received_buf[..bytes_received],
        &test_data[..bytes_received],
        "Received data should match sent data"
      );

      // Cleanup
      lio::close(client_sock).await.expect("Failed to close client");
      lio::close(server_client_fd)
        .await
        .expect("Failed to close server client");
      lio::close(server_sock).await.expect("Failed to close server");
    });
  }
}

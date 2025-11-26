use lio::{accept, bind, connect, listen, recv, send, socket};
use proptest::prelude::*;
use socket2::{Domain, Protocol, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;

#[test]
fn test_send_basic() {
  liten::block_on(async {
    // Setup server
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    println!("before");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    bind(server_sock, addr).await.expect("Failed to bind");
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

    // Accept and connect concurrently using liten::join!
    let accept_fut =
      async move { accept(server_sock).await.expect("Failed to accept") };

    let connect_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");
      client_sock
    };

    let (server_client_fd, client_sock) = liten::join!(accept_fut, connect_fut);

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
  });
}

#[test]
fn test_send_large_data() {
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

    let accept_fut =
      async move { accept(server_sock).await.expect("Failed to accept") };

    let connect_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");
      client_sock
    };

    let (server_client_fd, client_sock) = liten::join!(accept_fut, connect_fut);

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
  });
}

#[test]
#[ignore]
fn test_send_multiple() {
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

    let accept_fut =
      async move { accept(server_sock).await.expect("Failed to accept") };

    let connect_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");
      client_sock
    };

    let (server_client_fd, client_sock) = liten::join!(accept_fut, connect_fut);

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
  });
}

#[test]
#[ignore = "Problematic"]
fn test_send_with_flags() {
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

    let accept_fut =
      async move { accept(server_sock).await.expect("Failed to accept") };

    let connect_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");
      client_sock
    };

    let (server_client_fd, client_sock) = liten::join!(accept_fut, connect_fut);

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
  });
}

#[test]
fn test_send_on_closed_socket() {
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

    let accept_fut =
      async move { accept(server_sock).await.expect("Failed to accept") };

    let connect_fut = async {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");
      connect(client_sock, bound_addr)
        .await
        .expect("Failed to connect");
      client_sock
    };

    let (server_client_fd, client_sock) = liten::join!(accept_fut, connect_fut);

    // Close server end
    unsafe {
      libc::close(server_client_fd);
    }

    // Try to send after server closed
    let data = b"This should fail".to_vec();
    let (_result, _) = send(client_sock, data, None).await;

    // May succeed or fail depending on timing, but shouldn't crash
    unsafe {
      libc::close(client_sock);
      libc::close(server_sock);
    }
  });
}

#[test]
fn test_send_concurrent() {
  liten::block_on(async {
    // Test sending from multiple clients concurrently
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
        connect(client_sock, bound_addr)
          .await
          .expect("Failed to connect");
        client_sock
      };

      let (_server_client_fd, client_sock) =
        liten::join!(accept_fut, client_fut);

      // Send data
      let data = format!("Client {}", i).into_bytes();
      let (bytes_sent, _) = send(client_sock, data.clone(), None).await;
      let bytes_sent = bytes_sent.expect("Failed to send");

      assert_eq!(bytes_sent as usize, data.len());

      lio::close(client_sock).await.unwrap();
      lio::close(_server_client_fd).await.unwrap();
    }

    let _ = lio::close(server_sock).await.unwrap();
  });
}

proptest! {
  #[test]
  fn prop_test_send_arbitrary_data(
    data_size in 1usize..=8192,
    seed in any::<u64>(),
  ) {
    liten::block_on(async move {
      // Generate deterministic random data based on seed
      let test_data: Vec<u8> = (0..data_size)
        .map(|i| ((seed.wrapping_add(i as u64)) % 256) as u8)
        .collect();

      // Create server socket
      let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create server socket");

      // Bind to any available port on localhost
      let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
      bind(server_sock, addr)
        .await
        .expect("Failed to bind");

      // Get the actual bound address
      let bound_addr = unsafe {
        let mut addr_storage = MaybeUninit::<libc::sockaddr_in>::zeroed();
        let mut addr_len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        libc::getsockname(
          server_sock,
          addr_storage.as_mut_ptr() as *mut libc::sockaddr,
          &mut addr_len,
        );
        let sockaddr_in = addr_storage.assume_init();
        let port = u16::from_be(sockaddr_in.sin_port);
        format!("127.0.0.1:{}", port).parse::<SocketAddr>().unwrap()
      };

      // Start listening
      listen(server_sock, 128)
        .await
        .expect("Failed to listen");

      // Accept connection and connect client concurrently
      let (server_client_fd, client_sock) = liten::join!(
        async { accept(server_sock).await.expect("Accept failed") },
        async {
          let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .await
            .expect("Failed to create client socket");
          connect(sock, bound_addr)
            .await
            .expect("Connect failed");
          sock
        }
      );

      // Send data from client to server
      let (bytes_sent, returned_buf) = send(client_sock, test_data.clone(), None)
        .await;
      let bytes_sent = bytes_sent.expect("Send failed");

      assert_eq!(
        bytes_sent as usize,
        test_data.len(),
        "Send should return correct byte count"
      );
      assert_eq!(
        returned_buf,
        test_data,
        "Send returned buffer should match original data"
      );

      // Receive the data on server side to verify it was sent
      let recv_buf = vec![0u8; data_size];
      let (bytes_received, received_buf) = recv(server_client_fd, recv_buf, None)
        .await;
      let bytes_received = bytes_received.expect("Recv failed") as usize;

      assert!(
        bytes_received > 0,
        "Should receive at least some bytes"
      );
      assert_eq!(
        &received_buf[..bytes_received],
        &test_data[..bytes_received],
        "Received data should match sent data"
      );

      // Cleanup
      lio::close(client_sock).await.expect("Failed to close client");
      lio::close(server_client_fd).await.expect("Failed to close server client");
      lio::close(server_sock).await.expect("Failed to close server");
    });
  }
}

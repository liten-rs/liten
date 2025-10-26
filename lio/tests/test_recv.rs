use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use lio::{accept, bind, connect, listen, recv, send, socket};
use socket2::{Domain, Protocol, SockAddr, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

#[test]
#[ignore]
fn test_recv_basic() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    // Spawn server to accept and receive
    let server_handle = spawner
      .spawn_local_with_handle(async move {
        let mut client_addr_storage =
          MaybeUninit::<socket2::SockAddrStorage>::uninit();
        let mut client_addr_len =
          std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

        let client_fd = accept(
          server_sock,
          &mut client_addr_storage as *mut _,
          &mut client_addr_len,
        )
        .await
        .expect("Failed to accept");

        let buf = vec![0u8; 1024];
        let (bytes_received, received_buf) = recv(client_fd, buf, None).await;
        let bytes_received = bytes_received.expect("Failed to receive");

        (bytes_received, received_buf, client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    // Connect and send
    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");
    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    let send_data = b"Hello, Server!".to_vec();
    let (bytes_sent, _) = send(client_sock, send_data.clone(), None).await;
    bytes_sent.expect("Failed to send");

    let (bytes_received, received_buf, server_client_fd, server_sock) =
      server_handle.await;

    assert_eq!(bytes_received as usize, send_data.len());
    assert_eq!(&received_buf[..bytes_received as usize], send_data.as_slice());

    unsafe {
      libc::close(client_sock);
      libc::close(server_client_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore]
fn test_recv_large_data() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let server_handle = spawner
      .spawn_local_with_handle(async move {
        let mut client_addr_storage =
          MaybeUninit::<socket2::SockAddrStorage>::uninit();
        let mut client_addr_len =
          std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

        let client_fd = accept(
          server_sock,
          &mut client_addr_storage as *mut _,
          &mut client_addr_len,
        )
        .await
        .expect("Failed to accept");

        // Receive large data in chunks
        let mut total_received = Vec::new();
        loop {
          let buf = vec![0u8; 8192];
          let (bytes_received, received_buf) = recv(client_fd, buf, None).await;
          let bytes_received =
            bytes_received.expect("Failed to receive") as usize;

          if bytes_received == 0 {
            break;
          }

          total_received.extend_from_slice(&received_buf[..bytes_received]);

          if total_received.len() >= 1024 * 1024 {
            break;
          }
        }

        (total_received, client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");
    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    // Send large data (1MB)
    let large_data: Vec<u8> =
      (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    let (bytes_sent, _) = send(client_sock, large_data.clone(), None).await;
    bytes_sent.expect("Failed to send");

    let (received_data, server_client_fd, server_sock) = server_handle.await;

    assert_eq!(received_data.len(), large_data.len());
    assert_eq!(received_data, large_data);

    unsafe {
      libc::close(client_sock);
      libc::close(server_client_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore]
fn test_recv_partial() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let server_handle = spawner
      .spawn_local_with_handle(async move {
        let mut client_addr_storage =
          MaybeUninit::<socket2::SockAddrStorage>::uninit();
        let mut client_addr_len =
          std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

        let client_fd = accept(
          server_sock,
          &mut client_addr_storage as *mut _,
          &mut client_addr_len,
        )
        .await
        .expect("Failed to accept");

        // Receive with small buffer
        let buf = vec![0u8; 5];
        let (bytes_received, received_buf) = recv(client_fd, buf, None).await;
        let bytes_received = bytes_received.expect("Failed to receive");

        (bytes_received, received_buf, client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");
    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    let send_data = b"Hello, World!".to_vec();
    let (bytes_sent, _) = send(client_sock, send_data.clone(), None).await;
    bytes_sent.expect("Failed to send");

    let (bytes_received, received_buf, server_client_fd, server_sock) =
      server_handle.await;

    // Should only receive 5 bytes
    assert_eq!(bytes_received, 5);
    assert_eq!(&received_buf[..5], b"Hello");

    unsafe {
      libc::close(client_sock);
      libc::close(server_client_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore]
fn test_recv_multiple() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let server_handle = spawner
      .spawn_local_with_handle(async move {
        let mut client_addr_storage =
          MaybeUninit::<socket2::SockAddrStorage>::uninit();
        let mut client_addr_len =
          std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

        let client_fd = accept(
          server_sock,
          &mut client_addr_storage as *mut _,
          &mut client_addr_len,
        )
        .await
        .expect("Failed to accept");

        // Receive multiple times
        let mut messages = Vec::new();
        for _ in 0..3 {
          let buf = vec![0u8; 1024];
          let (bytes_received, received_buf) = recv(client_fd, buf, None).await;
          let bytes_received =
            bytes_received.expect("Failed to receive") as usize;
          messages.push(received_buf[..bytes_received].to_vec());
        }

        (messages, client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");
    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    // Send multiple messages
    for i in 0..3 {
      let data = format!("Message {}", i).into_bytes();
      let (bytes_sent, _) = send(client_sock, data, None).await;
      bytes_sent.expect("Failed to send");
      thread::sleep(Duration::from_millis(5));
    }

    let (messages, server_client_fd, server_sock) = server_handle.await;

    assert_eq!(messages.len(), 3);

    unsafe {
      libc::close(client_sock);
      libc::close(server_client_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore]
fn test_recv_with_flags() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let server_handle = spawner
      .spawn_local_with_handle(async move {
        let mut client_addr_storage =
          MaybeUninit::<socket2::SockAddrStorage>::uninit();
        let mut client_addr_len =
          std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

        let client_fd = accept(
          server_sock,
          &mut client_addr_storage as *mut _,
          &mut client_addr_len,
        )
        .await
        .expect("Failed to accept");

        let buf = vec![0u8; 1024];
        let (bytes_received, received_buf) =
          recv(client_fd, buf, Some(0)).await;
        let bytes_received =
          bytes_received.expect("Failed to receive with flags");

        (bytes_received, received_buf, client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");
    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    let send_data = b"Data with flags".to_vec();
    let (bytes_sent, _) = send(client_sock, send_data.clone(), None).await;
    bytes_sent.expect("Failed to send");

    let (bytes_received, received_buf, server_client_fd, server_sock) =
      server_handle.await;

    assert_eq!(bytes_received as usize, send_data.len());
    assert_eq!(&received_buf[..bytes_received as usize], send_data.as_slice());

    unsafe {
      libc::close(client_sock);
      libc::close(server_client_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore]
fn test_recv_on_closed() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let server_handle = spawner
      .spawn_local_with_handle(async move {
        let mut client_addr_storage =
          MaybeUninit::<socket2::SockAddrStorage>::uninit();
        let mut client_addr_len =
          std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;

        let client_fd = accept(
          server_sock,
          &mut client_addr_storage as *mut _,
          &mut client_addr_len,
        )
        .await
        .expect("Failed to accept");

        (client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");
    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    let (server_client_fd, server_sock) = server_handle.await;

    // Close client
    unsafe {
      libc::close(client_sock);
    }

    thread::sleep(Duration::from_millis(10));

    // Try to receive on closed connection
    let buf = vec![0u8; 1024];
    let (bytes_received, _) = recv(server_client_fd, buf, None).await;
    let bytes_received =
      bytes_received.expect("recv should succeed but return 0");

    // Should return 0 when connection is closed
    assert_eq!(bytes_received, 0);

    unsafe {
      libc::close(server_client_fd);
      libc::close(server_sock);
    }
  })
}

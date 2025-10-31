use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use lio::{accept, bind, connect, listen, socket};
use socket2::{Domain, Protocol, SockAddr, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

#[test]
#[ignore = "deadlocks"]
fn test_accept_basic() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
    // Create and setup server socket
    let server_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create server socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(server_sock, sock_addr).await.expect("Failed to bind");

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
    let accept_handle = spawner
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

    // Give accept time to start
    thread::sleep(Duration::from_millis(10));

    // Connect client
    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    // Wait for accept
    let (accepted_fd, server_sock) = accept_handle.await;

    assert!(accepted_fd >= 0, "Accepted fd should be valid");

    // Cleanup
    unsafe {
      libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore = "deadlocks"]
fn test_accept_multiple() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let num_clients = 5;
    let mut accepted_fds = Vec::new();
    let mut client_fds = Vec::new();

    for _ in 0..num_clients {
      // Spawn accept task
      let accept_handle = spawner.spawn_local_with_handle(async move {
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
      });

      thread::sleep(Duration::from_millis(5));

      // Connect client
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      connect(client_sock, SockAddr::from(bound_addr))
        .await
        .expect("Failed to connect");

      client_fds.push(client_sock);

      let (accepted_fd, _server_sock_returned) =
        accept_handle.expect("Failed to spawn task").await;
      accepted_fds.push(accepted_fd);
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
  })
}

#[test]
fn test_accept_with_client_info() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let accept_handle = spawner
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

        // Get client address info
        let client_addr = unsafe {
          SockAddr::new(client_addr_storage.assume_init_read(), client_addr_len)
        };

        (client_fd, client_addr, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create client socket");

    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect");

    let (accepted_fd, client_addr, server_sock) = accept_handle.await;

    // Verify client address is valid
    assert!(client_addr.as_socket_ipv4().is_some());

    // Cleanup
    unsafe {
      libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
fn test_accept_ipv6() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    let accept_handle = spawner
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
        .expect("Failed to accept IPv6");

        (client_fd, server_sock)
      })
      .expect("Failed to spawn task");

    thread::sleep(Duration::from_millis(10));

    let client_sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create IPv6 client socket");

    connect(client_sock, SockAddr::from(bound_addr))
      .await
      .expect("Failed to connect IPv6");

    let (accepted_fd, server_sock) = accept_handle.await;

    assert!(accepted_fd >= 0);

    // Cleanup
    unsafe {
      libc::close(client_sock);
      libc::close(accepted_fd);
      libc::close(server_sock);
    }
  })
}

#[test]
#[ignore = "not too sure how to handle concurrent poll registrations. Have to store fd and operation in some way in driver."]
fn test_accept_concurrent() {
  let mut pool = LocalPool::new();
  let spawner = pool.spawner();

  pool.run_until(async {
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

    // Spawn multiple concurrent accept tasks
    let accept_tasks: Vec<_> = (0..3)
      .map(|_| {
        spawner
          .spawn_local_with_handle(async move {
            let mut client_addr_storage =
              MaybeUninit::<socket2::SockAddrStorage>::uninit();
            let mut client_addr_len = std::mem::size_of::<
              socket2::SockAddrStorage,
            >() as libc::socklen_t;

            let client_fd = accept(
              server_sock,
              &mut client_addr_storage as *mut _,
              &mut client_addr_len,
            )
            .await
            .expect("Failed to accept");

            client_fd
          })
          .expect("Failed to spawn task")
      })
      .collect();

    thread::sleep(Duration::from_millis(20));

    // Connect multiple clients
    let mut client_fds = Vec::new();
    for _ in 0..3 {
      let client_sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .await
        .expect("Failed to create client socket");

      connect(client_sock, SockAddr::from(bound_addr))
        .await
        .expect("Failed to connect");

      client_fds.push(client_sock);
      thread::sleep(Duration::from_millis(5));
    }

    // Wait for all accepts
    let mut accepted_fds = Vec::new();
    for task in accept_tasks {
      let fd = task.await;
      accepted_fds.push(fd);
    }

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
  })
}

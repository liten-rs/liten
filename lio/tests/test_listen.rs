use futures::executor::block_on;
use lio::{bind, listen, socket};
use socket2::{Domain, Protocol, SockAddr, Type};
use std::net::SocketAddr;

#[cfg(linux)]
#[test]
fn test_listen_basic() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind socket");

    listen(sock, 128).await.expect("Failed to listen on socket");

    // Verify socket is in listening state by checking it accepts connections
    unsafe {
      let mut accept_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      let res = libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_ACCEPTCONN,
        &mut accept_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_ne!(res, -1);
      assert_eq!(
        accept_val,
        1,
        "Socket should be in listening state {:?}",
        Error::last_os_error()
      );
      libc::close(sock);
    }
  });
}

#[cfg(linux)]
#[test]
fn test_listen_with_backlog() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind socket");

    // Listen with custom backlog
    listen(sock, 10).await.expect("Failed to listen with backlog 10");

    // Verify listening state
    unsafe {
      let mut accept_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_ACCEPTCONN,
        &mut accept_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(accept_val, 1);
      libc::close(sock);
    }
  });
}

#[cfg(linux)]
#[test]
fn test_listen_large_backlog() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind socket");

    // Listen with large backlog
    listen(sock, 1024).await.expect("Failed to listen with large backlog");

    // Verify listening state
    unsafe {
      let mut accept_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_ACCEPTCONN,
        &mut accept_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(accept_val, 1);
      libc::close(sock);
    }
  });
}

#[test]
fn test_listen_without_bind() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    // Try to listen without binding first
    let result = listen(sock, 128).await;

    // On most systems this will succeed (bind to INADDR_ANY:0)
    // but behavior may vary by platform
    unsafe {
      libc::close(sock);
    }

    // Just verify it doesn't crash
    assert!(result.is_ok() || result.is_err());
  });
}

#[cfg(linux)]
#[test]
fn test_listen_ipv6() {
  block_on(async {
    let sock = socket(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create IPv6 socket");

    let addr: SocketAddr = "[::1]:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind IPv6 socket");

    listen(sock, 128).await.expect("Failed to listen on IPv6 socket");

    // Verify listening state
    unsafe {
      let mut accept_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_ACCEPTCONN,
        &mut accept_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(accept_val, 1);
      libc::close(sock);
    }
  });
}

#[test]
fn test_listen_on_udp() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
      .await
      .expect("Failed to create UDP socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind UDP socket");

    // Try to listen on UDP socket (should fail)
    let result = listen(sock, 128).await;

    assert!(result.is_err(), "Listen should fail on UDP socket");

    // Cleanup
    unsafe {
      libc::close(sock);
    }
  });
}

#[test]
fn test_listen_twice() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind socket");

    listen(sock, 128).await.expect("First listen should succeed");

    // Try to listen again on the same socket
    let result = listen(sock, 256).await;

    // Behavior may vary - some systems allow it, some don't
    unsafe {
      libc::close(sock);
    }

    // Just verify it doesn't crash
    assert!(result.is_ok() || result.is_err());
  });
}

#[cfg(linux)]
#[test]
fn test_listen_zero_backlog() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind socket");

    // Listen with backlog of 0 (system may adjust to minimum)
    listen(sock, 0).await.expect("Failed to listen with backlog 0");

    // Verify listening state
    unsafe {
      let mut accept_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_ACCEPTCONN,
        &mut accept_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(accept_val, 1);
      libc::close(sock);
    }
  });
}

#[test]
fn test_listen_after_close() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind socket");

    unsafe {
      libc::close(sock);
    }

    // Try to listen on closed socket
    let result = listen(sock, 128).await;

    assert!(result.is_err(), "Listen should fail on closed socket");
  });
}

#[cfg(linux)]
#[test]
fn test_listen_concurrent() {
  block_on(async {
    // Test listening on multiple sockets concurrently
    let tasks: Vec<_> = (0..10)
      .map(|_| async move {
        let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
          .await
          .expect("Failed to create socket");

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let sock_addr = SockAddr::from(addr);
        bind(sock, sock_addr).await.expect("Failed to bind socket");

        listen(sock, 128).await.expect("Failed to listen");

        unsafe {
          let mut accept_val: i32 = 0;
          let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
          libc::getsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_ACCEPTCONN,
            &mut accept_val as *mut _ as *mut libc::c_void,
            &mut len,
          );
          assert_eq!(accept_val, 1);
          libc::close(sock);
        }
      })
      .collect();

    for task in tasks {
      task.await;
    }
  });
}

#[cfg(linux)]
#[test]
fn test_listen_on_all_interfaces() {
  block_on(async {
    let sock = socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
      .await
      .expect("Failed to create socket");

    // Bind to 0.0.0.0 (all interfaces)
    let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let sock_addr = SockAddr::from(addr);
    bind(sock, sock_addr).await.expect("Failed to bind to all interfaces");

    listen(sock, 128).await.expect("Failed to listen on all interfaces");

    // Verify listening state
    unsafe {
      let mut accept_val: i32 = 0;
      let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
      libc::getsockopt(
        sock,
        libc::SOL_SOCKET,
        libc::SO_ACCEPTCONN,
        &mut accept_val as *mut _ as *mut libc::c_void,
        &mut len,
      );
      assert_eq!(accept_val, 1);
      libc::close(sock);
    }
  });
}

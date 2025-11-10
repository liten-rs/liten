use lio::close;
use lio::loom::test_utils::{block_on, model};
use std::ffi::CString;

#[test]
fn test_close_basic() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_close_basic.txt").unwrap();

      // Open a file
      let fd = unsafe {
        libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
          0o644,
        )
      };
      assert!(fd >= 0, "Failed to open file");

      // Close it
      close(fd).await.expect("Failed to close file descriptor");

      // Verify it's closed by trying to write to it (should fail)
      let result =
        unsafe { libc::write(fd, b"test".as_ptr() as *const libc::c_void, 4) };
      assert!(result < 0, "Writing to closed fd should fail");

      // Cleanup
      unsafe {
        libc::unlink(path.as_ptr());
      }
    })
  })
}

#[test]
fn test_close_after_read() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_close_after_read.txt").unwrap();

      // Create file with data
      unsafe {
        let fd = libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
          0o644,
        );
        libc::write(fd, b"test data".as_ptr() as *const libc::c_void, 9);
        libc::close(fd);
      }

      // Open for reading
      let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY) };
      assert!(fd >= 0);

      // Read some data
      let mut buf = vec![0u8; 9];
      unsafe {
        libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 9);
      }

      // Close it
      close(fd).await.expect("Failed to close file descriptor");

      // Cleanup
      unsafe {
        libc::unlink(path.as_ptr());
      }
    })
  })
}

#[test]
fn test_close_after_write() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_close_after_write.txt").unwrap();

      // Open file
      let fd = unsafe {
        libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
          0o644,
        )
      };

      // Write data
      unsafe {
        libc::write(fd, b"test data".as_ptr() as *const libc::c_void, 9);
      }

      // Close it
      close(fd).await.expect("Failed to close file descriptor");

      // Verify data was written
      unsafe {
        let read_fd = libc::open(path.as_ptr(), libc::O_RDONLY);
        let mut buf = vec![0u8; 9];
        let read_bytes =
          libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, 9);
        assert_eq!(read_bytes, 9);
        assert_eq!(&buf, b"test data");
        libc::close(read_fd);
        libc::unlink(path.as_ptr());
      }
    })
  })
}

// #[test]
// fn test_close_socket() {
//   model(|| {
//     block_on(async {
//       // Create a socket
//       let sock_fd =
//         unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
//       assert!(sock_fd >= 0, "Failed to create socket");
//
//       // Close it
//       close(sock_fd).await.expect("Failed to close socket");
//
//       // Verify it's closed by trying to use it (should fail)
//       let result = unsafe {
//         let addr = libc::sockaddr_in {
//           sin_family: libc::AF_INET as _,
//           sin_port: 0,
//           sin_addr: libc::in_addr { s_addr: 0 },
//           sin_zero: [0; 8],
//         };
//         libc::bind(
//           sock_fd,
//           &addr as *const _ as *const libc::sockaddr,
//           std::mem::size_of::<libc::sockaddr_in>() as u32,
//         )
//       };
//       assert!(result < 0, "Bind on closed socket should fail");
//     })
//   })
// }

#[test]
fn test_close_invalid_fd() {
  model(|| {
    block_on(async {
      // Try to close an invalid file descriptor
      let result = close(-1).await;
      assert!(result.is_err(), "Closing invalid fd should return error");
    })
  })
}

#[test]
fn test_close_already_closed() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_close_double.txt").unwrap();

      // Open a file
      let fd = unsafe {
        libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
          0o644,
        )
      };

      // Close it once
      close(fd).await.expect("First close should succeed");

      // Try to close again - should fail
      let result = close(fd).await;
      assert!(result.is_err(), "Closing already closed fd should fail");

      // Cleanup
      unsafe {
        libc::unlink(path.as_ptr());
      }
    })
  })
}

#[test]
fn test_close_concurrent() {
  model(|| {
    block_on(async {
      // Test closing multiple file descriptors sequentially
      let fds: Vec<_> = (0..10)
        .map(|i| {
          let path =
            CString::new(format!("/tmp/lio_test_close_concurrent_{}.txt", i))
              .unwrap();
          let fd = unsafe {
            libc::open(
              path.as_ptr(),
              libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
              0o644,
            )
          };
          (fd, path)
        })
        .collect();

      for (fd, path) in fds {
        close(fd).await.expect("Failed to close fd");
        unsafe {
          libc::unlink(path.as_ptr());
        }
      }
    })
  })
}

#[test]
fn test_close_pipe() {
  model(|| {
    block_on(async {
      // Create a pipe
      let mut pipe_fds = [0i32; 2];
      unsafe {
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
      }

      let read_fd = pipe_fds[0];
      let write_fd = pipe_fds[1];

      // Close both ends
      close(write_fd).await.expect("Failed to close write end");
      close(read_fd).await.expect("Failed to close read end");

      // Verify they're closed
      let result = unsafe {
        libc::write(write_fd, b"test".as_ptr() as *const libc::c_void, 4)
      };
      assert!(result < 0, "Writing to closed pipe should fail");
    })
  })
}

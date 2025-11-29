mod common;

use lio::{read, write};
use std::ffi::CString;
use std::sync::mpsc::{channel, sync_channel};
use std::time::Duration;

#[test]
fn test_callback_basic_read() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_callback_basic.txt").unwrap();
    let test_data = b"Hello, callback!";

    // Create test file
    let fd = unsafe {
      let fd = libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      );
      libc::write(
        fd,
        test_data.as_ptr() as *const libc::c_void,
        test_data.len(),
      );
      fd
    };

    // Use callback instead of awaiting
    let (tx, rx) = sync_channel(1);

    let buf = vec![0u8; 100];
    read(fd, buf, 0).when_done(move |(bytes_read, buffer)| {
      tx.send((bytes_read, buffer)).unwrap();
    });

    // Wait for callback to execute
    let (bytes_read, buffer) = rx
      .recv_timeout(Duration::from_secs(5))
      .expect("Callback was not invoked within timeout");
    let bytes_read = bytes_read.expect("Read failed") as usize;

    assert_eq!(bytes_read, test_data.len());
    assert_eq!(&buffer[..bytes_read], test_data);

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_callback_basic_write() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_callback_write.txt").unwrap();
    let test_data = b"Write with callback!";

    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      )
    };

    let (tx, rx) = sync_channel(1);

    write(fd, test_data.to_vec(), 0).when_done(
      move |(bytes_written, _buffer)| {
        tx.send(bytes_written).unwrap();
      },
    );

    // Wait for callback to execute
    let bytes_written = rx
      .recv_timeout(Duration::from_secs(5))
      .expect("Callback was not invoked within timeout");
    assert_eq!(bytes_written.expect("Write failed") as usize, test_data.len());

    // Verify data was written
    let mut read_buf = vec![0u8; 100];
    let bytes_read = unsafe {
      libc::lseek(fd, 0, libc::SEEK_SET);
      libc::read(fd, read_buf.as_mut_ptr() as *mut libc::c_void, read_buf.len())
    };
    assert_eq!(bytes_read as usize, test_data.len());
    assert_eq!(&read_buf[..bytes_read as usize], test_data);

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_callback_error_handling() {
  liten::block_on(async {
    // Try to read from an invalid fd
    let invalid_fd = -1;
    let (tx, rx) = sync_channel(1);

    let buf = vec![0u8; 100];
    read(invalid_fd, buf, 0).when_done(move |(bytes_read, _buffer)| {
      tx.send(bytes_read).unwrap();
    });

    // Wait for callback to execute
    let bytes_read = rx
      .recv_timeout(Duration::from_secs(5))
      .expect("Callback was not invoked within timeout");
    assert!(
      bytes_read.is_err(),
      "Expected error for invalid fd, got: {:?}",
      bytes_read
    );
  });
}

#[test]
fn test_callback_concurrent() {
  liten::block_on(async {
    // Test multiple concurrent operations with callbacks
    let (tx, rx) = channel();

    let tasks: Vec<_> = (0..10)
      .map(|i| {
        let tx_clone = tx.clone();
        async move {
          let path = CString::new(format!(
            "/tmp/lio_test_callback_concurrent_{}.txt",
            i
          ))
          .unwrap();
          let data = format!("Data for file {}", i);

          let fd = unsafe {
            let fd = libc::open(
              path.as_ptr(),
              libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
              0o644,
            );
            libc::write(fd, data.as_ptr() as *const libc::c_void, data.len());
            fd
          };

          let buf = vec![0u8; 100];
          let expected_data = data.clone();
          let path_clone = path.clone();

          read(fd, buf, 0).when_done(move |(bytes_read, buffer)| {
            let bytes_read = bytes_read.expect("Read failed") as usize;
            assert_eq!(bytes_read, expected_data.len());
            assert_eq!(&buffer[..bytes_read], expected_data.as_bytes());

            // Cleanup
            unsafe {
              libc::close(fd);
              libc::unlink(path_clone.as_ptr());
            }

            tx_clone.send(i).unwrap();
          });
        }
      })
      .collect();

    for task in tasks {
      task.await;
    }

    // Drop the original sender so we can check when all are done
    drop(tx);

    // Collect all results
    let mut results: Vec<_> = rx.iter().collect();
    results.sort();

    // Verify all callbacks were invoked
    assert_eq!(results.len(), 10, "Not all callbacks were invoked");
    assert_eq!(results, (0..10).collect::<Vec<_>>());
  });
}

#[test]
fn test_callback_vs_future_mutually_exclusive() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_callback_exclusive.txt").unwrap();
    let test_data = b"Mutual exclusion test";

    let fd = unsafe {
      let fd = libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      );
      libc::write(
        fd,
        test_data.as_ptr() as *const libc::c_void,
        test_data.len(),
      );
      fd
    };

    // Create an operation progress
    let buf = vec![0u8; 100];
    let progress = read(fd, buf, 0);

    // Use callback - this takes ownership, so future can't be polled
    let (tx, rx) = sync_channel(1);

    progress.when_done(move |(bytes_read, buffer)| {
      tx.send((bytes_read, buffer)).unwrap();
    });

    // At this point, we can't poll the future because when_done took ownership

    // Wait for callback
    let (bytes_read, buffer) = rx
      .recv_timeout(Duration::from_secs(5))
      .expect("Callback should have been invoked");

    let bytes_read = bytes_read.expect("Read failed") as usize;
    assert_eq!(bytes_read, test_data.len());
    assert_eq!(&buffer[..bytes_read], test_data);

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_callback_with_detach() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_callback_detach.txt").unwrap();
    let test_data = b"Detach test";

    let fd = unsafe {
      let fd = libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      );
      libc::write(
        fd,
        test_data.as_ptr() as *const libc::c_void,
        test_data.len(),
      );
      fd
    };

    let (tx, rx) = sync_channel(1);

    let buf = vec![0u8; 100];
    read(fd, buf, 0).when_done(move |_result| {
      // Just mark as invoked, don't care about result
      tx.send(()).unwrap();
    });

    // Wait for callback
    rx.recv_timeout(Duration::from_secs(5))
      .expect("Callback should have been invoked");

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_callback_preserves_buffer_ownership() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_callback_ownership.txt").unwrap();
    let test_data = b"Buffer ownership test";

    let fd = unsafe {
      let fd = libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      );
      libc::write(
        fd,
        test_data.as_ptr() as *const libc::c_void,
        test_data.len(),
      );
      fd
    };

    let (tx, rx) = sync_channel(1);

    let buf = vec![0u8; 100];
    read(fd, buf, 0).when_done(move |(bytes_read, buffer)| {
      // Verify we got ownership of the buffer
      let bytes_read = bytes_read.expect("Read failed") as usize;
      assert_eq!(&buffer[..bytes_read], test_data);

      // Send buffer to verify it lives beyond callback
      tx.send(buffer).unwrap();
    });

    // Wait for callback
    let buffer = rx
      .recv_timeout(Duration::from_secs(5))
      .expect("Buffer should be received");
    assert_eq!(&buffer[..test_data.len()], test_data);

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

use lio::loom::test_utils::block_on;
use lio::{loom::test_utils::model, read};
use std::ffi::CString;

#[test]
fn test_read_basic() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_basic.txt").unwrap();

      // Create and write test data using libc
      let test_data = b"Hello, read test!";
      let fd = unsafe {
        let fd = libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
          0o644,
        );
        libc::write(
          fd,
          test_data.as_ptr() as *const libc::c_void,
          test_data.len(),
        );
        libc::close(fd);
        libc::open(path.as_ptr(), libc::O_RDONLY)
      };

      // Read using lio
      let buf = vec![0u8; 1024];
      let (bytes_read, result) = read(fd, buf, 0).await;
      let bytes_read = bytes_read.expect("Failed to read") as usize;

      assert_eq!(bytes_read, test_data.len());
      assert_eq!(&result[..bytes_read], test_data);

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_with_offset() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_offset.txt").unwrap();

      // Create test file with data
      let test_data = b"0123456789ABCDEFGHIJ";
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

      // Read from offset 5
      let buf = vec![0u8; 10];
      let (bytes_read, result) = read(fd, buf, 5).await;
      let bytes_read = bytes_read.expect("Failed to read with offset") as usize;

      assert_eq!(bytes_read, 10);
      assert_eq!(&result[..bytes_read], b"56789ABCDE");

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_partial() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_partial.txt").unwrap();

      // Create file with limited data
      let test_data = b"Short";
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

      // Try to read more than available
      let buf = vec![0u8; 1024];
      let (bytes_read, result) = read(fd, buf, 0).await;
      let bytes_read = bytes_read.expect("Failed to read") as usize;

      assert_eq!(bytes_read, test_data.len());
      assert_eq!(&result[..bytes_read], test_data);

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_empty_file() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_empty.txt").unwrap();

      // Create empty file
      let fd = unsafe {
        libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_RDONLY | libc::O_TRUNC,
          0o644,
        )
      };

      // Read from empty file
      let buf = vec![0u8; 100];
      let (bytes_read, _) = read(fd, buf, 0).await;
      let bytes_read = bytes_read.expect("Failed to read from empty file");

      assert_eq!(bytes_read, 0);

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_beyond_eof() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_beyond_eof.txt").unwrap();

      let test_data = b"Data";
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

      // Read from offset beyond file size
      let buf = vec![0u8; 100];
      let (bytes_read, _) = read(fd, buf, 1000).await;
      let bytes_read = bytes_read.expect("Failed to read beyond EOF");

      assert_eq!(bytes_read, 0);

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_large_buffer() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_large.txt").unwrap();

      // Create large data (1MB)
      let large_data: Vec<u8> =
        (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
      let fd = unsafe {
        let fd = libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
          0o644,
        );
        libc::write(
          fd,
          large_data.as_ptr() as *const libc::c_void,
          large_data.len(),
        );
        fd
      };

      // Read it back
      let buf = vec![0u8; 1024 * 1024];
      let (bytes_read, result) = read(fd, buf, 0).await;
      let bytes_read =
        bytes_read.expect("Failed to read large buffer") as usize;

      assert_eq!(bytes_read, large_data.len());
      assert_eq!(&result[..bytes_read], large_data.as_slice());

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_multiple_sequential() {
  model(|| {
    block_on(async {
      let path = CString::new("/tmp/lio_test_read_sequential.txt").unwrap();

      let test_data = b"ABCDEFGHIJKLMNOP";
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

      // Read in chunks at different offsets
      let buf1 = vec![0u8; 5];
      let (bytes_read1, result1) = read(fd, buf1, 0).await;
      assert_eq!(bytes_read1.unwrap() as usize, 5);
      assert_eq!(&result1[..5], b"ABCDE");

      let buf2 = vec![0u8; 5];
      let (bytes_read2, result2) = read(fd, buf2, 5).await;
      assert_eq!(bytes_read2.unwrap() as usize, 5);
      assert_eq!(&result2[..5], b"FGHIJ");

      let buf3 = vec![0u8; 6];
      let (bytes_read3, result3) = read(fd, buf3, 10).await;
      assert_eq!(bytes_read3.unwrap() as usize, 6);
      assert_eq!(&result3[..6], b"KLMNOP");

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    });
  });
}

#[test]
fn test_read_concurrent() {
  model(|| {
    block_on(async {
      // Test multiple concurrent read operations on different files
      let tasks: Vec<_> = (0..10)
        .map(|i| async move {
          let path =
            CString::new(format!("/tmp/lio_test_read_concurrent_{}.txt", i))
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
          let (bytes_read, result) = read(fd, buf, 0).await;
          let bytes_read = bytes_read.expect("Failed to read") as usize;

          assert_eq!(bytes_read, data.len());
          assert_eq!(&result[..bytes_read], data.as_bytes());

          unsafe {
            libc::close(fd);
            libc::unlink(path.as_ptr());
          }
        })
        .collect();

      for task in tasks {
        task.await;
      }
    });
  });
}

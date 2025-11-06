/// write in append mode is not tested since `pwrite` doesn't support it.
use lio::loom::test_utils::block_on;
use lio::write;
use std::ffi::CString;

#[test]
fn test_write_basic() {
  block_on(async {
    let path = CString::new("/tmp/lio_test_write_basic.txt").unwrap();

    // Create file
    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
        0o644,
      )
    };

    // Write data
    let data = b"Hello, write test!".to_vec();
    let (bytes_written, returned_buf) = write(fd, data.clone(), 0).await;
    let bytes_written = bytes_written.expect("Failed to write") as usize;

    assert_eq!(bytes_written, data.len());
    assert_eq!(returned_buf, data);

    // Verify by reading back
    let mut verify_buf = vec![0u8; data.len()];
    unsafe {
      let read_fd = libc::open(path.as_ptr(), libc::O_RDONLY);
      let read_bytes = libc::read(
        read_fd,
        verify_buf.as_mut_ptr() as *mut libc::c_void,
        data.len(),
      );
      assert_eq!(read_bytes as usize, data.len());
      assert_eq!(verify_buf, data);
      libc::close(read_fd);
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_write_with_offset() {
  block_on(async {
    let path = CString::new("/tmp/lio_test_write_offset.txt").unwrap();

    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      )
    };

    // Write initial data
    let data1 = b"AAAAAAAAAA".to_vec();
    let (bytes_written, _) = write(fd, data1, 0).await;
    bytes_written.expect("Failed to write initial data");

    // Write at offset
    let data2 = b"BBB".to_vec();
    let (bytes_written, _) = write(fd, data2, 5).await;
    bytes_written.expect("Failed to write at offset");

    // Verify
    let mut verify_buf = vec![0u8; 10];
    unsafe {
      libc::lseek(fd, 0, libc::SEEK_SET);
      libc::read(fd, verify_buf.as_mut_ptr() as *mut libc::c_void, 10);
      assert_eq!(&verify_buf[0..5], b"AAAAA");
      assert_eq!(&verify_buf[5..8], b"BBB");
      assert_eq!(&verify_buf[8..10], b"AA");
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_write_empty_buffer() {
  block_on(async {
    let path = CString::new("/tmp/lio_test_write_empty.txt").unwrap();

    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
        0o644,
      )
    };

    // Write empty buffer
    let data = Vec::new();
    let (bytes_written, _) = write(fd, data, 0).await;
    let bytes_written = bytes_written.expect("Failed to write empty buffer");

    assert_eq!(bytes_written, 0);

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_write_large_buffer() {
  block_on(async {
    let path = CString::new("/tmp/lio_test_write_large.txt").unwrap();

    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
        0o644,
      )
    };

    // Write large buffer (1MB)
    let large_data: Vec<u8> =
      (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    let (bytes_written, returned_buf) = write(fd, large_data.clone(), 0).await;
    let bytes_written =
      bytes_written.expect("Failed to write large buffer") as usize;

    assert_eq!(bytes_written, large_data.len());
    assert_eq!(returned_buf, large_data);

    // Verify file size
    unsafe {
      let mut stat: libc::stat = std::mem::zeroed();
      libc::fstat(fd, &mut stat);
      assert_eq!(stat.st_size as usize, large_data.len());
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_write_multiple_sequential() {
  block_on(async {
    let path = CString::new("/tmp/lio_test_write_sequential.txt").unwrap();

    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
        0o644,
      )
    };

    // Write multiple chunks at different offsets
    let data1 = b"AAAAA".to_vec();
    let (bytes_written1, _) = write(fd, data1, 0).await;
    assert_eq!(bytes_written1.unwrap(), 5);

    let data2 = b"BBBBB".to_vec();
    let (bytes_written2, _) = write(fd, data2, 5).await;
    assert_eq!(bytes_written2.unwrap(), 5);

    let data3 = b"CCCCC".to_vec();
    let (bytes_written3, _) = write(fd, data3, 10).await;
    assert_eq!(bytes_written3.unwrap(), 5);

    // Verify
    let mut verify_buf = vec![0u8; 15];
    unsafe {
      let read_fd = libc::open(path.as_ptr(), libc::O_RDONLY);
      libc::read(read_fd, verify_buf.as_mut_ptr() as *mut libc::c_void, 15);
      assert_eq!(verify_buf, b"AAAAABBBBBCCCCC");
      libc::close(read_fd);
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_write_overwrite() {
  block_on(async {
    let path = CString::new("/tmp/lio_test_write_overwrite.txt").unwrap();

    let fd = unsafe {
      libc::open(
        path.as_ptr(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
        0o644,
      )
    };

    // Write initial data
    let data1 = b"XXXXXXXXXX".to_vec();
    let (bytes_written, _) = write(fd, data1, 0).await;
    bytes_written.expect("Failed to write initial data");

    // Overwrite part of it
    let data2 = b"YYY".to_vec();
    let (bytes_written, _) = write(fd, data2, 3).await;
    bytes_written.expect("Failed to overwrite data");

    // Verify
    let mut verify_buf = vec![0u8; 10];
    unsafe {
      libc::lseek(fd, 0, libc::SEEK_SET);
      libc::read(fd, verify_buf.as_mut_ptr() as *mut libc::c_void, 10);
      println!("{:?}", String::from_utf8(verify_buf.clone()));
      assert_eq!(&verify_buf, b"XXXYYYXXXX");
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_write_concurrent() {
  block_on(async {
    // Test multiple concurrent write operations on different files
    let tasks: Vec<_> = (0..10)
      .map(|i| async move {
        let path =
          CString::new(format!("/tmp/lio_test_write_concurrent_{}.txt", i))
            .unwrap();
        let data = format!("Task {}", i).into_bytes();

        let fd = unsafe {
          libc::open(
            path.as_ptr(),
            libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
            0o644,
          )
        };

        let (bytes_written, returned_buf) = write(fd, data.clone(), 0).await;
        let bytes_written = bytes_written.expect("Failed to write") as usize;

        assert_eq!(bytes_written, data.len());
        assert_eq!(returned_buf, data);

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
}

mod common;

use lio::read;
use proptest::prelude::*;
use std::ffi::CString;

#[test]
fn test_read_large_buffer() {
  liten::block_on(async {
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
    let bytes_read = bytes_read.expect("Failed to read large buffer") as usize;

    assert_eq!(bytes_read, large_data.len());
    assert_eq!(&result[..bytes_read], large_data.as_slice());

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_read_concurrent() {
  liten::block_on(async {
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
}

proptest! {
  #[test]
  fn prop_test_read_arbitrary_data_and_offsets(
    data_size in 0usize..=8192,
    read_offset in 0i64..=4096,
    buffer_size in 0usize..=4096,
    seed in any::<u64>(),
  ) {
    let result = liten::block_on(async move {

      // Generate deterministic random data based on seed
      let test_data: Vec<u8> = (0..data_size)
        .map(|i| ((seed.wrapping_add(i as u64)) % 256) as u8)
        .collect();

      // Create unique test file path
      let path = common::make_temp_path("read", seed);

      // Write test data to file
      let fd = unsafe {
        let fd = libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
          0o644,
        );
        assert!(fd >= 0, "Failed to create test file");

        let written = libc::write(
          fd,
          test_data.as_ptr() as *const libc::c_void,
          test_data.len(),
        );
        assert_eq!(written as usize, test_data.len(), "Failed to write all data");
        fd
      };

      // Perform the read operation
      let buf = vec![0u8; buffer_size];
      let (bytes_read_result, result_buf) = lio::read(fd, buf, read_offset).await;

      let test_result = (|| -> Result<(), TestCaseError> {
        let bytes_read = bytes_read_result
          .map_err(|e| TestCaseError::fail(format!("Read operation failed unexpectedly: {}", e)))?;
        let bytes_read = bytes_read as usize;

        // Calculate expected bytes to read
        let file_size = test_data.len();
        let offset = read_offset as usize;

        if offset >= file_size {
          // Reading beyond EOF should return 0 bytes
          if bytes_read != 0 {
            return Err(TestCaseError::fail(format!(
              "Reading beyond EOF should return 0 bytes, got {}", bytes_read
            )));
          }
        } else {
          // Calculate how many bytes we expect to read
          let available_bytes = file_size - offset;
          let expected_bytes = std::cmp::min(buffer_size, available_bytes);

          if bytes_read != expected_bytes {
            return Err(TestCaseError::fail(format!(
              "Read should return min(buffer_size={}, available_bytes={})={}, got {}",
              buffer_size, available_bytes, expected_bytes, bytes_read
            )));
          }

          // Verify the data matches what we wrote
          let expected_data = &test_data[offset..offset + bytes_read];
          if &result_buf[..bytes_read] != expected_data {
            return Err(TestCaseError::fail(
              "Read data should match written data at offset".to_string()
            ));
          }
        }
        Ok(())
      })();

      // Cleanup
      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }

      test_result
    });

    result?;
  }
}

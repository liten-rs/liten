/// write in append mode is not tested since `pwrite` doesn't support it.
mod common;

use lio::write;
use proptest::prelude::*;
use std::ffi::CString;

#[test]
fn test_write_large_buffer() {
  liten::block_on(async {
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
fn test_write_concurrent() {
  liten::block_on(async {
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

proptest! {
  #[test]
  fn prop_test_write_arbitrary_data_and_offsets(
    data_size in 0usize..=8192,
    write_offset in 0i64..=4096,
    seed in any::<u64>(),
  ) {
    let result = liten::block_on(async move {
      // Generate deterministic random data based on seed
      let test_data: Vec<u8> = (0..data_size)
        .map(|i| ((seed.wrapping_add(i as u64)) % 256) as u8)
        .collect();

      // Create unique test file path
      let path = common::make_temp_path("write", seed);

      // Create file for writing
      let fd = unsafe {
        libc::open(
          path.as_ptr(),
          libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
          0o644,
        )
      };

      let test_result = (|| -> Result<(), TestCaseError> {
        if fd < 0 {
          return Err(TestCaseError::fail("Failed to create test file".to_string()));
        }

        // If writing at an offset, we need to create a file with enough space
        // Fill the file with zeros up to write_offset + data_size
        if write_offset > 0 {
          let zeros = vec![0u8; write_offset as usize];
          unsafe {
            let written = libc::write(
              fd,
              zeros.as_ptr() as *const libc::c_void,
              zeros.len(),
            );
            if written < 0 || written as usize != zeros.len() {
              return Err(TestCaseError::fail("Failed to write initial zeros".to_string()));
            }
          }
        }

        // Perform the async write operation
        let (write_result, returned_buf) = liten::block_on(write(fd, test_data.clone(), write_offset));

        let bytes_written = write_result
          .map_err(|e| TestCaseError::fail(format!("Write operation failed: {}", e)))?;

        // Verify bytes written
        if bytes_written as usize != test_data.len() {
          return Err(TestCaseError::fail(format!(
            "Write should return data_size={}, got {}",
            test_data.len(), bytes_written
          )));
        }

        // Verify returned buffer matches original
        if returned_buf != test_data {
          return Err(TestCaseError::fail(
            "Returned buffer should match original data".to_string()
          ));
        }

        // Read back and verify the data was written correctly
        let mut read_buf = vec![0u8; test_data.len()];
        unsafe {
          let read_bytes = libc::pread(
            fd,
            read_buf.as_mut_ptr() as *mut libc::c_void,
            test_data.len(),
            write_offset,
          );

          if read_bytes < 0 {
            return Err(TestCaseError::fail("Failed to read back data".to_string()));
          }

          if read_bytes as usize != test_data.len() {
            return Err(TestCaseError::fail(format!(
              "Read back {} bytes, expected {}",
              read_bytes, test_data.len()
            )));
          }

          if read_buf != test_data {
            return Err(TestCaseError::fail(
              "Read data does not match written data".to_string()
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

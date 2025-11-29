#![cfg(feature = "high")]
use lio::openat;
use std::ffi::CString;

#[test]
#[ignore = "Problematic"]
fn test_openat_create_file() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_openat_create.txt").unwrap();

    // Open/create file for writing
    let fd = openat(
      libc::AT_FDCWD,
      path.clone(),
      libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC | libc::O_RDONLY,
    )
    .await
    .expect("Failed to create file");

    assert!(fd >= 0, "File descriptor should be valid");

    // Close and cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
#[ignore = "need to figure out permission stuff"]
fn test_openat_read_only() {
  liten::block_on(async {
    let path = CString::new("./test_openat_readonly_testfile.txt").unwrap();

    // Create file first
    let fd_create = openat(
      libc::AT_FDCWD,
      path.clone(),
      libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
    )
    .await
    .expect("Failed to create file");
    unsafe { libc::close(fd_create) };

    // Open for reading
    let fd = openat(libc::AT_FDCWD, path.clone(), libc::O_RDONLY)
      .await
      .expect("Failed to open file for reading");

    assert!(fd >= 0, "File descriptor should be valid");

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_openat_read_write() {
  liten::block_on(async {
    let path = CString::new("/tmp/lio_test_openat_rdwr.txt").unwrap();

    // Open for reading and writing
    let fd = openat(
      libc::AT_FDCWD,
      path.clone(),
      libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
    )
    .await
    .expect("Failed to open file for read/write");

    assert!(fd >= 0, "File descriptor should be valid");

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::unlink(path.as_ptr());
    }
  });
}

#[test]
fn test_openat_nonexistent_file() {
  liten::block_on(async {
    let path =
      CString::new("/tmp/lio_test_nonexistent_file_12345.txt").unwrap();

    // Try to open non-existent file without O_CREAT
    let result = openat(libc::AT_FDCWD, path, libc::O_RDONLY).await;

    assert!(result.is_err(), "Should fail to open non-existent file");
  });
}

#[test]
fn test_openat_with_directory_fd() {
  liten::block_on(async {
    // Open /tmp directory
    let tmp_path = CString::new("/tmp").unwrap();
    let dir_fd = unsafe {
      libc::open(tmp_path.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY)
    };
    assert!(dir_fd >= 0, "Failed to open /tmp directory");

    // Open file relative to directory fd
    let file_path = CString::new("lio_test_openat_dirfd.txt").unwrap();
    let fd = openat(
      dir_fd,
      file_path.clone(),
      libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
    )
    .await
    .expect("Failed to open file with directory fd");

    assert!(fd >= 0, "File descriptor should be valid");

    // Cleanup
    unsafe {
      libc::close(fd);
      libc::close(dir_fd);
      // Full path for cleanup
      let full_path = CString::new("/tmp/lio_test_openat_dirfd.txt").unwrap();
      libc::unlink(full_path.as_ptr());
    }
  });
}

#[test]
fn test_openat_concurrent() {
  liten::block_on(async {
    // Test multiple sequential openat operations
    for i in 0..10 {
      let path =
        CString::new(format!("/tmp/lio_test_openat_concurrent_{}.txt", i))
          .unwrap();

      let fd = openat(
        libc::AT_FDCWD,
        path.clone(),
        libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
      )
      .await
      .expect("Failed to open file");

      assert!(fd >= 0);

      unsafe {
        libc::close(fd);
        libc::unlink(path.as_ptr());
      }
    }
  });
}

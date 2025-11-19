use std::ffi::CString;

/// Utility function to create a unique temporary file path for proptest tests.
/// Returns a CString path that includes the thread ID and a unique value to avoid conflicts.
pub fn make_temp_path(test_name: &str, unique_value: u64) -> CString {
  CString::new(format!(
    "/tmp/lio_proptest_{}_{:?}_{}.txt",
    test_name,
    std::thread::current().id(),
    unique_value
  ))
  .expect("Failed to create CString path")
}

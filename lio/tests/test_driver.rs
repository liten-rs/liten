#[test]
fn test_driver() {
  lio::init();
  // Note: Removed lio::exit() to prevent shutting down the shared Driver
  // during parallel test execution
}

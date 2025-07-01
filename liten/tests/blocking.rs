use std::{thread, time};

use liten::blocking::unblock;

#[liten::test]
async fn simple() {
  // Define a blocking operation
  let blocking_operation = || {
    thread::sleep(time::Duration::from_millis(100)); // Simulate a blocking operation
    42 // Return some result
  };

  // Call the unblock function with the blocking operation
  let result = unblock(blocking_operation).await;

  // Assert that the result is as expected
  assert_eq!(result, 42);
}

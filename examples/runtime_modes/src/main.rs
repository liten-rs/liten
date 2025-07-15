use liten::{runtime::Runtime, task, time::sleep};
use std::time::Duration;

#[liten::main]
async fn main() {
  println!("=== Runtime Mode Examples ===\n");

  // Example 1: Single-threaded runtime
  println!("1. Single-threaded runtime:");
  Runtime::single_threaded().block_on(async {
    println!("  Running in single-threaded mode");
    println!(
      "  Worker thread count: {}",
      Runtime::single_threaded().worker_thread_count()
    );
    println!(
      "  Execution mode: {:?}",
      Runtime::single_threaded().execution_mode()
    );

    // Spawn some tasks
    let handle1 = task::spawn(async {
      println!("    Task 1: Starting");
      sleep(Duration::from_millis(100)).await;
      println!("    Task 1: Completed");
      1
    });

    let handle2 = task::spawn(async {
      println!("    Task 2: Starting");
      sleep(Duration::from_millis(50)).await;
      println!("    Task 2: Completed");
      2
    });

    let (result1, result2) = (handle1.await, handle2.await);
    println!("    Results: {} and {}", result1, result2);
  });
}


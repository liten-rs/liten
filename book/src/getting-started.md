# Getting Started

This guide will help you set up Liten and write your first async application.

## Installation

Add Liten to your `Cargo.toml`:

```toml
[dependencies]
liten = "0.1.0"
```

## Your First Async Program

Create a new Rust project and replace the contents of `src/main.rs`:

```rust
use liten::{Runtime, task, time};

#[liten::main]
async fn main() {
    println!("Starting Liten application...");
    
    // Spawn a background task
    let handle = task::spawn(async {
        time::sleep(std::time::Duration::from_millis(100)).await;
        "Task completed!"
    });
    
    // Do some work in the main task
    println!("Main task working...");
    time::sleep(std::time::Duration::from_millis(50)).await;
    
    // Wait for the background task
    let result = handle.await.unwrap();
    println!("{}", result);
    
    println!("Application finished!");
}
```

Run your program:

```bash
cargo run
```

You should see output like:
```
Starting Liten application...
Main task working...
Task completed!
Application finished!
```

## Understanding the Code

### `#[liten::main]`
This attribute macro transforms your `main` function into an async function that runs on the Liten runtime.

### `task::task::spawn`
Creates a new task that runs concurrently with the current task. Returns a handle that can be awaited.

### `time::time::sleep`
Pauses execution for the specified duration, allowing other tasks to run.

### `.await`
Waits for an async operation to complete.

## Next Steps

Now that you have a basic understanding, explore:

- [Tasks](./tasks.md) - Learn about task management and spawning
- [Synchronization](./sync.md) - Understand how to coordinate between tasks
- [Time and Timers](./time.md) - Work with time-based operations
- [Blocking Operations](./blocking.md) - Handle CPU-intensive work


## Configuration

Liten can be configured when building the runtime:

```rust
use liten::Runtime;

#[liten::main]
async fn main() {
    // Configure the runtime
    let runtime = Runtime::builder()
        .num_workers(4)  // Set number of worker threads
        .block_on(async {
            // Your async code here
            println!("Running on configured runtime");
        });
}
```

Ready to explore more advanced features? Check out the [Core Concepts](./runtime.md) section! 
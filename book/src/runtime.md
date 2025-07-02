# Runtime

The Liten runtime is the core component that manages async execution. It provides the infrastructure for task scheduling, worker thread management, and async/await support.

## Runtime Overview

The runtime consists of several key components:

- **Worker Threads**: Execute async tasks
- **Task Scheduler**: Manages task distribution and execution
- **Timer Wheel**: Handles time-based operations
- **Thread Pool**: Manages blocking operations

## Creating a Runtime

### Basic Runtime

```rust
use liten::Runtime;

// The runtime is automatically created and managed
#[liten::main]
async fn main() {
    // Your async code here
    println!("Running on Liten runtime");
    
    println!("Async operation completed");
}
```

### Custom Runtime Configuration

```rust
use liten::Runtime;

fn main() {
    // Create a custom runtime
    let runtime = Runtime::builder()
        .num_workers(4)  // Set number of worker threads
        .build();
    
    // Run async code on the runtime
    runtime.block_on(async {
        println!("Running on custom runtime");
        
        // Your async code here
        println!("Async operation completed");
    });
}
```

## Runtime Configuration

### Worker Threads

Configure the number of worker threads:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(8)  // Use 8 worker threads
        .build();
    
    runtime.block_on(async {
        // This will run on one of the 8 worker threads
        println!("Running on configured runtime");
    });
}
```

### Work Stealing

Control work stealing behavior:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(4)
        .disable_work_stealing()  // Disable work stealing
        .build();
    
    runtime.block_on(async {
        // Tasks will only run on the thread that spawned them
        println!("Work stealing disabled");
    });
}
```

### Thread Names

Set names for worker threads:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(4)
        .worker_thread_name("liten-worker")  // Set worker thread names
        .build();
    
    runtime.block_on(async {
        println!("Running on named worker threads");
    });
}
```

## Runtime Lifecycle

### Startup

When a runtime starts:

1. **Worker threads are created**: Each worker thread runs a task processing loop
2. **Scheduler is initialized**: The work-stealing scheduler is set up
3. **Timer wheel is started**: Time-based operations are enabled
4. **Thread pool is created**: Blocking operations are ready

### Shutdown

When a runtime shuts down:

1. **Tasks are cancelled**: Pending tasks are dropped
2. **Worker threads stop**: All worker threads terminate
3. **Resources are cleaned up**: Memory and other resources are freed

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(2)
        .build();
    
    runtime.block_on(async {
        // Runtime is active here
        println!("Runtime is running");
    });
    
    // Runtime is automatically shut down when it goes out of scope
    println!("Runtime has shut down");
}
```

## Task Scheduling

### Work Stealing Scheduler

Liten uses a work-stealing scheduler for efficient task distribution:

```rust
use liten::{Runtime, task};

fn main() {
    let runtime = Runtime::builder()
        .num_workers(4)
        .build();
    
    runtime.block_on(async {
        // Spawn many tasks
        let handles: Vec<_> = (0..100).map(|i| {
            task::spawn(async move {
                // Each task will be assigned to a worker thread
                // If a thread is busy, tasks can be stolen by other threads
                println!("Task {} running", i);
                i * i
            })
        }).collect();
        
        // Wait for all tasks
        for handle in handles {
            let result = handle.await.unwrap();
            println!("Task result: {}", result);
        }
    });
}
```

### Task Distribution

Tasks are distributed based on:

- **Spawn location**: Tasks are initially assigned to the thread that spawned them
- **Load balancing**: Busy threads can steal tasks from other threads
- **Task size**: Small tasks may be batched together

## Performance Tuning

### Worker Thread Count

Choose the right number of worker threads:

```rust
use liten::Runtime;

fn main() {
    // For CPU-bound workloads
    let cpu_runtime = Runtime::builder()
        .num_workers(num_cpus::get())  // One thread per CPU core
        .build();
    
    // For I/O-bound workloads
    let io_runtime = Runtime::builder()
        .num_workers(num_cpus::get() * 2)  // More threads for I/O
        .build();
    
    // For mixed workloads
    let mixed_runtime = Runtime::builder()
        .num_workers(num_cpus::get() + 2)  // Balanced approach
        .build();
}
```

### Memory Configuration

Configure memory usage:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(4)
        // Memory configuration options would go here
        .build();
    
    runtime.block_on(async {
        // Your async code
    });
}
```

## Runtime Metrics

### Task Count

Monitor the number of active tasks:

```rust
use liten::{Runtime, task};

fn main() {
    let runtime = Runtime::builder()
        .num_workers(2)
        .build();
    
    runtime.block_on(async {
        // Spawn some tasks
        let handles: Vec<_> = (0..10).map(|i| {
            task::spawn(async move {
                // Simulate some work
                std::thread::sleep(std::time::Duration::from_millis(100));
                i
            })
        }).collect();
        
        // Wait for all tasks
        for handle in handles {
            let result = handle.await.unwrap();
            println!("Task {} completed", result);
        }
    });
}
```

### Worker Thread Utilization

Monitor worker thread activity:

```rust
use liten::{Runtime, task, time};

fn main() {
    let runtime = Runtime::builder()
        .num_workers(4)
        .build();
    
    runtime.block_on(async {
        // Create a mix of short and long tasks
        let short_tasks: Vec<_> = (0..20).map(|i| {
            task::spawn(async move {
                time::sleep(std::time::Duration::from_millis(10)).await;
                format!("Short task {}", i)
            })
        }).collect();
        
        let long_tasks: Vec<_> = (0..4).map(|i| {
            task::spawn(async move {
                time::sleep(std::time::Duration::from_millis(100)).await;
                format!("Long task {}", i)
            })
        }).collect();
        
        // Wait for all tasks
        for handle in short_tasks {
            let result = handle.await.unwrap();
            println!("{} completed", result);
        }
        
        for handle in long_tasks {
            let result = handle.await.unwrap();
            println!("{} completed", result);
        }
    });
}
```

## Error Handling

### Runtime Errors

Handle runtime-level errors:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(1)
        .build();
    
    runtime.block_on(async {
        // Handle panics in tasks
        let handle = task::spawn(async {
            // This might panic
            if rand::random::<bool>() {
                panic!("Random panic");
            }
            "Success"
        });
        
        match handle.await {
            Ok(result) => println!("Task succeeded: {}", result),
            Err(_) => println!("Task panicked"),
        }
    });
}
```

### Resource Exhaustion

Handle resource exhaustion:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(1)  // Limited resources
        .build();
    
    runtime.block_on(async {
        // Spawn many tasks to test resource limits
        let handles: Vec<_> = (0..1000).map(|i| {
            task::spawn(async move {
                // Very small task
                i
            })
        }).collect();
        
        // Wait for all tasks
        for handle in handles {
            let result = handle.await.unwrap();
            if result % 100 == 0 {
                println!("Processed {} tasks", result);
            }
        }
    });
}
```

## Advanced Configuration

### Custom Scheduler

Configure custom scheduling behavior:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(4)
        // Custom scheduler configuration would go here
        .build();
    
    runtime.block_on(async {
        // Your async code with custom scheduling
        println!("Running with custom scheduler");
    });
}
```

### Runtime Hooks

Add runtime lifecycle hooks:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(2)
        // Runtime hooks would go here
        .build();
    
    runtime.block_on(async {
        // Your async code
        println!("Runtime is active");
    });
    
    // Runtime cleanup happens automatically
}
```

## Best Practices

### Runtime Sizing

Choose appropriate runtime configuration:

```rust
use liten::Runtime;

fn main() {
    // For small applications
    let small_runtime = Runtime::builder()
        .num_workers(2)
        .build();
    
    // For medium applications
    let medium_runtime = Runtime::builder()
        .num_workers(num_cpus::get())
        .build();
    
    // For large applications
    let large_runtime = Runtime::builder()
        .num_workers(num_cpus::get() * 2)
        .build();
}
```

### Resource Management

Manage runtime resources properly:

```rust
use liten::Runtime;

fn main() {
    // Create runtime with appropriate resources
    let runtime = Runtime::builder()
        .num_workers(4)
        .build();
    
    // Use the runtime
    runtime.block_on(async {
        // Your async code
    });
    
    // Runtime is automatically cleaned up
}
```

### Error Recovery

Implement error recovery strategies:

```rust
use liten::Runtime;

fn main() {
    let runtime = Runtime::builder()
        .num_workers(2)
        .build();
    
    runtime.block_on(async {
        // Implement retry logic for failed operations
        let mut attempts = 0;
        let max_attempts = 3;
        
        while attempts < max_attempts {
            match risky_operation().await {
                Ok(result) => {
                    println!("Operation succeeded: {}", result);
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    println!("Attempt {} failed: {}", attempts, e);
                    if attempts < max_attempts {
                        time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
    });
}

async fn risky_operation() -> Result<String, String> {
    // Simulate a risky operation
    if rand::random::<bool>() {
        Ok("Success".to_string())
    } else {
        Err("Operation failed".to_string())
    }
}
```

## Next Steps

Now that you understand the runtime, explore:
- [Tasks](./tasks.md) - Learn how tasks are scheduled and executed
- [Synchronization](./sync.md) - Understand how tasks coordinate
- [Time and Timers](./time.md) - See how time-based operations work
- [Blocking Operations](./blocking.md) - Learn about thread pool integration 
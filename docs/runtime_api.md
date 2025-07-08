# Runtime API

The Liten runtime provides flexible execution modes to suit different application needs. You can choose between single-threaded and multi-threaded execution based on your requirements.

## Architecture

The runtime is built with a modular scheduler architecture:

- **Single-threaded Scheduler**: Runs all tasks on the main thread
- **Multi-threaded Scheduler**: Distributes tasks across worker threads
- **Unified Scheduler**: Automatically chooses the appropriate scheduler based on configuration

Both schedulers implement the same `SchedulerTrait` interface, ensuring consistent behavior regardless of the execution mode.

## Overview

The runtime supports two main execution modes:

- **Single-threaded**: All tasks run on the main thread, providing deterministic execution and lower overhead
- **Multi-threaded**: Tasks are distributed across multiple worker threads, providing better performance for CPU-bound workloads

## Basic Usage

### Single-threaded Runtime

For simple applications or when you need deterministic execution:

```rust
use liten::runtime::Runtime;

Runtime::single_threaded()
    .block_on(async {
        // Your async code here
        println!("Running in single-threaded mode");
    });
```

### Multi-threaded Runtime

For applications that benefit from parallel execution:

```rust
use liten::runtime::Runtime;

Runtime::multi_threaded()
    .block_on(async {
        // Your async code here
        println!("Running in multi-threaded mode");
    });
```

### Direct Scheduler Usage

You can also use the schedulers directly for more control:

```rust
use liten::runtime::Runtime;

// Single-threaded convenience method
Runtime::block_on_single_threaded(async {
    // Your async code here
});

// Multi-threaded convenience method
Runtime::block_on_multi_threaded(async {
    // Your async code here
});
```

## Builder Pattern

The runtime uses a builder pattern for configuration:

```rust
use liten::runtime::Runtime;

// Start with a builder
let builder = Runtime::builder();

// Configure it
let configured = builder
    .multi_threaded_mode()
    .worker_threads(4)
    .disable_work_stealing();

// Use it
configured.block_on(async {
    // Your async code here
});
```

## Configuration Options

### Execution Mode

Set the execution mode explicitly:

```rust
// Single-threaded mode
Runtime::builder()
    .single_threaded_mode()
    .block_on(async { /* ... */ });

// Multi-threaded mode
Runtime::builder()
    .multi_threaded_mode()
    .block_on(async { /* ... */ });
```

### Worker Threads

Configure the number of worker threads for multi-threaded execution:

```rust
Runtime::multi_threaded()
    .worker_threads(8)  // Use 8 worker threads
    .block_on(async { /* ... */ });
```

The default is to use the number of available CPU cores.

### Work Stealing

Control work stealing behavior in multi-threaded mode:

```rust
// Disable work stealing (default: enabled)
Runtime::multi_threaded()
    .disable_work_stealing()
    .block_on(async { /* ... */ });

// Explicitly enable work stealing
Runtime::multi_threaded()
    .enable_work_stealing()
    .block_on(async { /* ... */ });
```

## Runtime Information

You can inspect the runtime configuration:

```rust
let builder = Runtime::multi_threaded().worker_threads(4);

println!("Execution mode: {:?}", builder.execution_mode());
println!("Worker thread count: {}", builder.worker_thread_count());
```

## Scheduler Architecture

### Single-threaded Scheduler

The single-threaded scheduler (`SingleThreadedScheduler`) runs all tasks on the main thread:

- **Pros**: Deterministic execution, lower overhead, simpler debugging
- **Cons**: No parallelism, limited to single CPU core performance
- **Use case**: Simple applications, debugging, constrained environments

### Multi-threaded Scheduler

The multi-threaded scheduler (`MultiThreadedScheduler`) distributes tasks across worker threads:

- **Pros**: Better performance for CPU-bound tasks, true parallelism
- **Cons**: Higher overhead, non-deterministic execution, more complex debugging
- **Use case**: High-performance applications, servers, CPU-intensive workloads

### Unified Scheduler

The unified scheduler (`Scheduler`) automatically chooses the appropriate implementation:

```rust
use liten::runtime::scheduler::Scheduler;

// Automatically uses single-threaded scheduler
Scheduler.block_on(async { /* ... */ }, RuntimeBuilder::single_threaded());

// Automatically uses multi-threaded scheduler
Scheduler.block_on(async { /* ... */ }, RuntimeBuilder::multi_threaded());
```

## When to Use Each Mode

### Single-threaded Mode

Use single-threaded mode when:

- You need deterministic execution
- Your application is simple and doesn't benefit from parallelism
- You want to minimize resource usage
- You're debugging async code and want predictable behavior
- You're running in a constrained environment

### Multi-threaded Mode

Use multi-threaded mode when:

- You have CPU-bound workloads that benefit from parallelism
- You're handling multiple concurrent operations
- You want to maximize throughput
- You're building a server or high-performance application

## Examples

### Basic Task Spawning

```rust
use liten::{runtime::Runtime, task, time::sleep};
use std::time::Duration;

Runtime::multi_threaded()
    .block_on(async {
        let handle1 = task::spawn(async {
            sleep(Duration::from_millis(100)).await;
            "Task 1 completed"
        });

        let handle2 = task::spawn(async {
            sleep(Duration::from_millis(50)).await;
            "Task 2 completed"
        });

        let (result1, result2) = (handle1.await, handle2.await);
        println!("{} and {}", result1, result2);
    });
```

### Parallel Processing

```rust
use liten::{runtime::Runtime, task};
use std::sync::Arc;

Runtime::multi_threaded()
    .worker_threads(4)
    .block_on(async {
        let data = Arc::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let mut handles = vec![];

        for i in 0..4 {
            let data = data.clone();
            let handle = task::spawn(async move {
                // Process a chunk of data
                let start = i * 2;
                let end = (i + 1) * 2;
                data[start..end].iter().sum::<i32>()
            });
            handles.push(handle);
        }

        let results: Vec<i32> = futures::future::join_all(handles).await;
        let total: i32 = results.iter().sum();
        println!("Total: {}", total);
    });
```

### Deterministic Single-threaded Execution

```rust
use liten::{runtime::Runtime, task, time::sleep};
use std::time::Duration;

Runtime::single_threaded()
    .block_on(async {
        // Tasks will execute in a predictable order
        let handle1 = task::spawn(async {
            println!("Task 1 starts");
            sleep(Duration::from_millis(10)).await;
            println!("Task 1 ends");
            1
        });

        let handle2 = task::spawn(async {
            println!("Task 2 starts");
            sleep(Duration::from_millis(5)).await;
            println!("Task 2 ends");
            2
        });

        let (result1, result2) = (handle1.await, handle2.await);
        println!("Results: {} and {}", result1, result2);
    });
```

### Direct Scheduler Usage

```rust
use liten::runtime::Runtime;

// Using convenience methods
let result1 = Runtime::block_on_single_threaded(async {
    // This runs on the main thread
    "single-threaded result"
});

let result2 = Runtime::block_on_multi_threaded(async {
    // This runs with worker threads
    "multi-threaded result"
});
```

## Performance Considerations

### Single-threaded Mode

- **Pros**: Lower overhead, deterministic execution, simpler debugging
- **Cons**: No parallelism, limited to single CPU core performance

### Multi-threaded Mode

- **Pros**: Better performance for CPU-bound tasks, true parallelism
- **Cons**: Higher overhead, non-deterministic execution, more complex debugging

### Work Stealing

Work stealing helps balance load across worker threads:

- **Enabled** (default): Better load balancing, but some overhead
- **Disabled**: Lower overhead, but potential load imbalance

## Migration from Old API

If you're migrating from the old API:

```rust
// Old API
Runtime::builder().num_workers(4).block_on(async { /* ... */ });

// New API - equivalent
Runtime::multi_threaded().worker_threads(4).block_on(async { /* ... */ });

// New API - single-threaded (not available in old API)
Runtime::single_threaded().block_on(async { /* ... */ });
```

## Thread Safety

- **Single-threaded mode**: Tasks run on the main thread, so `Send` is not required
- **Multi-threaded mode**: Tasks may run on any worker thread, so `Send` is required

```rust
use liten::runtime::Runtime;

// This works in single-threaded mode
Runtime::single_threaded().block_on(async {
    let non_send = std::rc::Rc::new(42);
    // Use non_send here
});

// This requires Send in multi-threaded mode
Runtime::multi_threaded().block_on(async {
    // All spawned tasks must be Send
    liten::task::spawn(async {
        // This closure must be Send
    });
});
```

## Internal Architecture

The runtime is organized into several modules:

- `runtime/mod.rs`: Main runtime API and builder
- `runtime/scheduler/mod.rs`: Unified scheduler that chooses implementation
- `runtime/scheduler/single_threaded.rs`: Single-threaded scheduler implementation
- `runtime/scheduler/multi_threaded.rs`: Multi-threaded scheduler implementation
- `runtime/scheduler/trait.rs`: Common trait for schedulers
- `runtime/scheduler/worker/`: Worker thread implementation for multi-threaded mode

This modular design allows for easy testing, maintenance, and potential future extensions. 
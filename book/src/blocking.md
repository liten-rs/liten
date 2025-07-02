# Blocking Operations

Liten provides utilities for handling blocking operations that would otherwise block the async runtime. This includes CPU-intensive work, file system operations, and other blocking I/O.

## Thread Pool

Liten includes a thread pool for executing blocking operations without blocking the async runtime.

### Basic Usage

```rust
use liten::blocking;

#[liten::main]
async fn main() {
    // Spawn a blocking task
    let result = blocking::spawn(|| {
        // This runs on a separate thread
        let mut sum = 0;
        for i in 0..1000000 {
            sum += i;
        }
        sum
    }).await;
    
    println!("Sum: {}", result);
}
```

### CPU-Intensive Work

Use the thread pool for CPU-intensive operations:

```rust
use liten::blocking;

#[liten::main]
async fn main() {
    let handles: Vec<_> = (0..4).map(|i| {
        blocking::spawn(move || {
            // Simulate CPU-intensive work
            let mut result = 0;
            for j in 0..1000000 {
                result += j * (i + 1);
            }
            (i, result)
        })
    }).collect();
    
    for handle in handles {
        let (task_id, result) = handle.await;
        println!("Task {} completed with result: {}", task_id, result);
    }
}
```

### Blocking I/O Operations

Use the thread pool for blocking I/O:

```rust
use liten::blocking;
use std::fs;

#[liten::main]
async fn main() {
    let file_content = blocking::spawn(|| {
        // This blocking I/O runs on a separate thread
        fs::read_to_string("example.txt")
    }).await;
    
    match file_content {
        Ok(content) => println!("File content: {}", content),
        Err(e) => println!("Error reading file: {}", e),
    }
}
```

## File System Operations

Liten provides async wrappers around file system operations.

### Reading Files

```rust
use liten::fs;

#[liten::main]
async fn main() {
    // Read a file asynchronously
    match fs::read_to_string("example.txt").await {
        Ok(content) => println!("File content: {}", content),
        Err(e) => println!("Error reading file: {}", e),
    }
}
```

### Writing Files

```rust
use liten::fs;

#[liten::main]
async fn main() {
    // Write to a file asynchronously
    match fs::write("output.txt", "Hello, Liten!").await {
        Ok(_) => println!("File written successfully"),
        Err(e) => println!("Error writing file: {}", e),
    }
}
```


## Combining Blocking and Async

### Mixed Workloads

You can combine blocking and async operations:

```rust
use liten::{blocking, task, time};

#[liten::main]
async fn main() {
    // Spawn multiple tasks with different types of work
    let async_task = task::spawn(async {
        time::sleep(std::time::Duration::from_millis(100)).await;
        "Async task completed"
    });
    
    let blocking_task = blocking::spawn(|| {
        // CPU-intensive work
        let mut sum = 0;
        for i in 0..1000000 {
            sum += i;
        }
        sum
    });
    
    // Wait for both tasks
    let async_result = async_task.await.unwrap();
    let blocking_result = blocking_task.await;
    
    println!("Async: {}, Blocking: {}", async_result, blocking_result);
}
```

### Pipeline Processing

Create pipelines that mix async and blocking operations:

```rust
use liten::{blocking, task, sync};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::oneshot::channel();
    
    // Stage 1: Async data generation
    let generator = task::spawn(async move {
        let data: Vec<i32> = (0..1000).collect();
        sender.send(data).unwrap();
    });
    
    // Stage 2: Blocking processing
    let processor = blocking::spawn(async move {
        let data = receiver.await.unwrap();
        
        // CPU-intensive processing
        data.into_iter()
            .map(|x| x * x)
            .filter(|&x| x % 2 == 0)
            .sum::<i32>()
    });
    
    // Wait for the pipeline to complete
    generator.await.unwrap();
    let result = processor.await;
    
    println!("Pipeline result: {}", result);
}
```

## Thread Pool Configuration

### Custom Thread Pool

You can configure the thread pool for your specific needs:

```rust
use liten::blocking;

#[liten::main]
async fn main() {
    // The thread pool is automatically managed by the runtime
    // You can control the number of threads via runtime configuration
    
    // For CPU-intensive work, use more threads
    let cpu_work = blocking::spawn(|| {
        // Heavy computation
        let mut result = 0;
        for i in 0..1000000 {
            result += i * i;
        }
        result
    });
    
    // For I/O work, fewer threads might be sufficient
    let io_work = blocking::spawn(|| {
        // I/O operation
        std::thread::sleep(std::time::Duration::from_millis(100));
        "I/O completed"
    });
    
    let cpu_result = cpu_work.await;
    let io_result = io_work.await;
    
    println!("CPU: {}, I/O: {}", cpu_result, io_result);
}
```

## Best Practices

### When to Use Blocking Operations

**Use blocking operations for:**
- CPU-intensive computations
- Blocking I/O operations
- Long-running synchronous code
- Operations that would block the async runtime

**Use async operations for:**
- I/O operations that can be made async
- Network operations
- Timer-based operations
- Lightweight computations

### Thread Pool Sizing

The optimal thread pool size depends on your workload:

```rust
use liten::blocking;

#[liten::main]
async fn main() {
    // For CPU-bound work, use number of CPU cores
    let cpu_tasks: Vec<_> = (0..num_cpus::get()).map(|i| {
        blocking::spawn(move || {
            // CPU-intensive work
            let mut sum = 0;
            for j in 0..1000000 {
                sum += j * (i + 1);
            }
            sum
        })
    }).collect();
    
    // For I/O-bound work, you can use more threads
    let io_tasks: Vec<_> = (0..10).map(|i| {
        blocking::spawn(move || {
            // Simulate I/O work
            std::thread::sleep(std::time::Duration::from_millis(100));
            format!("I/O task {} completed", i)
        })
    }).collect();
    
    // Wait for all tasks
    for handle in cpu_tasks {
        let result = handle.await;
        println!("CPU result: {}", result);
    }
    
    for handle in io_tasks {
        let result = handle.await;
        println!("{}", result);
    }
}
```

### Error Handling

Always handle errors in blocking operations:

```rust
use liten::blocking;

#[liten::main]
async fn main() {
    let result = blocking::spawn(|| {
        // This might panic
        if rand::random::<bool>() {
            panic!("Random panic in blocking task");
        }
        "Success"
    }).await;
    
    match result {
        Ok(value) => println!("Blocking task succeeded: {}", value),
        Err(_) => println!("Blocking task panicked"),
    }
}
```

### Resource Management

Be careful with resources in blocking operations:

```rust
use liten::blocking;
use std::sync::Arc;

#[liten::main]
async fn main() {
    let shared_resource = Arc::new(SomeResource::new());
    
    let handles: Vec<_> = (0..5).map(|i| {
        let resource = shared_resource.clone();
        blocking::spawn(move || {
            // Use the shared resource
            resource.do_work(i);
        })
    }).collect();
    
    for handle in handles {
        handle.await;
    }
    
    // Resource is automatically cleaned up when Arc goes out of scope
}
```

## Performance Considerations

### Thread Pool Overhead

Each blocking task has some overhead:
- Thread creation (if needed)
- Task scheduling
- Context switching

For very small operations, consider if blocking is necessary.

## Next Steps

Now that you understand blocking operations, explore:
- [Synchronization](./sync.md) - Coordinate between blocking and async operations
- [Time and Timers](./time.md) - Add timeouts to blocking operations
- [Examples](./examples/concurrent.md) - See blocking operations in real-world scenarios 
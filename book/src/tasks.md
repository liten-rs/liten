# Tasks

Tasks are the fundamental unit of concurrency in Liten. They represent an async computation that can run concurrently with other tasks.

## Spawning Tasks

### Basic Task Spawning

Use `task::spawn` to create a new task:

```rust
use liten::task;

#[liten::main]
async fn main() {
    let handle = task::spawn(async {
        // This runs in a separate task
        println!("Hello from spawned task!");
        42
    });
    
    let result = handle.await.unwrap();
    println!("Task returned: {}", result);
}
```

### Task Builder

For more control over task creation, use the builder pattern:

```rust
use liten::task;

#[liten::main]
async fn main() {
    let handle = task::builder()
        .name("my-task")  // Set a name for debugging
        .build(async {
            println!("Named task running");
            "success"
        });
    
    let result = handle.await.unwrap();
    println!("Result: {}", result);
}
```

## Task Handles

When you spawn a task, you get a `TaskHandle` that allows you to interact with the task:

```rust
use liten::task;

#[liten::main]
async fn main() {
    let handle = task::spawn(async {
        // Simulate some work
        std::thread::sleep(std::time::Duration::from_millis(100));
        "work completed"
    });
    
    // The handle implements IntoFuture, so you can await it
    match handle.await {
        Ok(result) => println!("Task succeeded: {}", result),
        Err(_) => println!("Task panicked"),
    }
}
```

## Task Lifecycle

### Creation
Tasks are created when you call `task::spawn` or use the builder. They are immediately scheduled for execution.

### Execution
Tasks run on worker threads in the runtime. They can be:
- **Running**: Currently executing
- **Ready**: Ready to run but waiting for a worker thread
- **Waiting**: Blocked on an async operation

### Completion
Tasks complete when:
- The async block finishes normally
- The task panics
- The task is cancelled (if cancellation is implemented)

## Best Practices

### Task Granularity

**Good**: Spawn tasks for independent units of work
```rust
#[liten::main]
async fn main() {
    let handles: Vec<_> = (0..10).map(|i| {
        task::spawn(async move {
            // Each task does independent work
            format!("Task {} completed", i)
        })
    }).collect();
    
    for handle in handles {
        println!("{}", handle.await.unwrap());
    }
}
```

**Avoid**: Spawning too many tiny tasks
```rust
// Don't do this - too many small tasks
for i in 0..10000 {
    task::spawn(async move {
        println!("{}", i);  // Very small amount of work
    });
}
```

### Error Handling

Always handle potential task panics:

```rust
use liten::task;

#[liten::main]
async fn main() {
    let handle = task::spawn(async {
        // This might panic
        if rand::random::<bool>() {
            panic!("Random panic!");
        }
        "success"
    });
    
    match handle.await {
        Ok(result) => println!("Success: {}", result),
        Err(_) => println!("Task panicked"),
    }
}
```

### Resource Management

Tasks can hold resources. Make sure to clean them up:

```rust
use liten::task;
use std::sync::Arc;

#[liten::main]
async fn main() {
    let resource = Arc::new(SomeResource::new());
    
    let handle = task::spawn({
        let resource = resource.clone();
        async move {
            // Use the resource
            resource.do_something().await;
        }
    });
    
    // Wait for the task to complete
    handle.await.unwrap();
    
    // Resource is automatically cleaned up when Arc goes out of scope
}
```

## Advanced Patterns

### Task Coordination

Use channels to coordinate between tasks:

```rust
use liten::{task, sync};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::oneshot::channel();
    
    let worker = task::spawn(async move {
        // Do some work
        let result = expensive_computation().await;
        sender.send(result).unwrap();
    });
    
    let coordinator = task::spawn(async move {
        let result = receiver.await.unwrap();
        println!("Received result: {}", result);
    });
    
    // Wait for both tasks
    worker.await.unwrap();
    coordinator.await.unwrap();
}
```

### Task Cancellation

While Liten doesn't have built-in task cancellation, you can implement it using shared state:

```rust
use liten::task;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[liten::main]
async fn main() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();
    
    let task = task::spawn(async move {
        loop {
            if cancelled_clone.load(Ordering::Relaxed) {
                break "cancelled";
            }
            
            // Do some work
            time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });
    
    // Cancel the task after 1 second
    time::sleep(std::time::Duration::from_secs(1)).await;
    cancelled.store(true, Ordering::Relaxed);
    
    let result = task.await.unwrap();
    println!("Task result: {}", result);
}
```

## Performance Considerations

### Task Overhead
Each task has some overhead:
- Memory allocation for the task structure
- Scheduling overhead
- Context switching

For very small computations, consider batching work instead of spawning individual tasks.

### Work Stealing
Liten uses a work-stealing scheduler, which means:
- Tasks are initially assigned to the thread that spawned them
- If a thread runs out of work, it can "steal" tasks from other threads
- This provides good load balancing automatically

### Task Pinning
Tasks are pinned in memory during execution. This means:
- They cannot be moved between threads once started
- The async block and its captured variables stay in place
- This is necessary for async/await to work correctly

## Debugging Tasks

### Task Names
Use the builder to name tasks for easier debugging:

```rust
let handle = task::builder()
    .name("database-query")
    .build(async {
        // Your database query here
    });
```

### Task IDs
Each task has a unique ID that can be used for debugging:

```rust
use liten::task;

#[liten::main]
async fn main() {
    let handle = task::spawn(async {
        let task_id = task::current().id();
        println!("Running task with ID: {:?}", task_id);
    });
    
    handle.await.unwrap();
}
```

## Next Steps

Now that you understand tasks, explore:
- [Synchronization](./sync.md) - Coordinate between tasks
- [Time and Timers](./time.md) - Add time-based behavior to tasks
- [Blocking Operations](./blocking.md) - Handle CPU-intensive work in tasks 
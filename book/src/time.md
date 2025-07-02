# Time and Timers

Liten provides time-based utilities for scheduling, delays, and timer management. These features help you build applications that need to work with time-based operations. Liten's timer provides millisecond-level accuracy.

## Sleep

The most basic time operation is `sleep`, which pauses execution for a specified duration.

### Basic Sleep

```rust
use liten::{time, task};

#[liten::main]
async fn main() {
    println!("Starting...");
    
    // Sleep for 1 second
    time::sleep(std::time::Duration::from_secs(1)).await;
    
    println!("1 second has passed!");
}
```

### Sleep with Different Durations

```rust
use liten::time;

#[liten::main]
async fn main() {
    println!("Sleeping for different durations...");
    
    // Sleep for milliseconds
    time::sleep(std::time::Duration::from_millis(150)).await;
    println!("100ms passed");
    
    // Sleep for seconds
    time::sleep(std::time::Duration::from_seconds(2)).await;
    println!("2s passed");
    
    // Sleep for hours
    time::sleep(std::time::Duration::from_hours(2)).await;
    println!("2 hours passed");
}
```

### Sleep in Tasks

Sleep works well with task spawning:

```rust
use liten::{time, task};

#[liten::main]
async fn main() {
    let handles: Vec<_> = (0..5).map(|i| {
        task::spawn(async move {
            // Each task sleeps for a different duration
            let sleep_duration = std::time::Duration::from_millis(100 * (i + 1));
            time::sleep(sleep_duration).await;
            println!("Task {} completed after {:?}", i, sleep_duration);
            i
        })
    }).collect();
    
    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        println!("Task {} finished", result);
    }
}
```


```rust
use liten::time;
use std::time::Instant;

#[liten::main]
async fn main() {
    let start = Instant::now();
    
    // Sleep for exactly 100ms
    time::sleep(std::time::Duration::from_millis(100)).await;
    
    let elapsed = start.elapsed();
    println!("Requested: 100ms, Actual: {:?}", elapsed);
    
    // The actual time should be very close to 100ms
    assert!(elapsed >= std::time::Duration::from_millis(100));
}
```

## Time-Based Patterns

### Periodic Tasks

Create tasks that run periodically:

```rust
use liten::{time, task};

#[liten::main]
async fn main() {
    let periodic_task = task::spawn(async {
        let mut count = 0;
        loop {
            time::sleep(std::time::Duration::from_millis(500)).await;
            count += 1;
            println!("Periodic task run #{}", count);
            
            if count >= 5 {
                break;
            }
        }
    });
    
    periodic_task.await.unwrap();
    println!("Periodic task completed");
}
```


### Timer Precision

- **Millisecond precision**: Liten provides millisecond-level timer precision
- **Low overhead**: The timer wheel implementation is designed for minimal CPU usage
- **Scalability**: Can handle thousands of concurrent timers efficiently


### Timer Memory Usage

Each timer consumes a small amount of memory. For applications with many timers:

- Reuse timers when possible
- Cancel timers when they're no longer needed
- Consider using a timer pool for high-frequency timer operations

## Best Practices

### Use Appropriate Durations

Choose the right duration for your use case:

```rust
use liten::time;

#[liten::main]
async fn main() {
    // For UI updates: 16ms (60 FPS)
    time::sleep(std::time::Duration::from_millis(16)).await;
    
    // For polling: 100ms-1s
    time::sleep(std::time::Duration::from_millis(100)).await;
    
    // For background tasks: 1s-1min
    time::sleep(std::time::Duration::from_secs(1)).await;
    
    // For maintenance tasks: 1min+
    time::sleep(std::time::Duration::from_secs(60)).await;
}
```

### Combine with Other Primitives

Time operations work well with other Liten primitives:

```rust
use liten::{time, sync, task};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::oneshot::channel();
    
    // Spawn a task that sends a value after a delay
    task::spawn(async move {
        time::sleep(std::time::Duration::from_millis(100)).await;
        sender.send("Delayed message").unwrap();
    });
    
    // Wait for the message
    let message = receiver.await.unwrap();
    println!("Received: {}", message);
}
```

### Error Handling

Handle cases where time operations might fail:

```rust
use liten::time;

#[liten::main]
async fn main() {
    // Sleep for a very long duration
    let long_duration = std::time::Duration::from_secs(3600); // 1 hour
    
    // In a real application, you might want to handle cancellation
    time::sleep(long_duration).await;
    
    println!("Slept for a long time");
}
```

## Advanced Usage

### Custom Timer Implementation

You can build custom timer functionality:

```rust
use liten::{time, task, sync};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

struct CustomTimer {
    cancelled: Arc<AtomicBool>,
}

impl CustomTimer {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
    
    async fn sleep_for(&self, duration: std::time::Duration) -> bool {
        let cancelled = self.cancelled.clone();
        
        // Sleep in small chunks to allow cancellation
        let chunk_size = std::time::Duration::from_millis(10);
        let mut remaining = duration;
        
        while remaining > std::time::Duration::ZERO {
            if cancelled.load(Ordering::Relaxed) {
                return false; // Cancelled
            }
            
            let sleep_duration = std::cmp::min(remaining, chunk_size);
            time::sleep(sleep_duration).await;
            remaining -= sleep_duration;
        }
        
        true // Completed
    }
    
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[liten::main]
async fn main() {
    let timer = CustomTimer::new();
    let timer_clone = timer.clone();
    
    // Spawn a task that cancels the timer after 500ms
    task::spawn(async move {
        time::sleep(std::time::Duration::from_millis(500)).await;
        timer_clone.cancel();
    });
    
    // Try to sleep for 2 seconds
    let completed = timer.sleep_for(std::time::Duration::from_secs(2)).await;
    
    if completed {
        println!("Timer completed");
    } else {
        println!("Timer was cancelled");
    }
}
```

## Next Steps

Now that you understand time and timers, explore:
- [Blocking Operations](./blocking.md) - Combine time operations with CPU-intensive work
- [Synchronization](./sync.md) - Use timeouts with synchronization primitives
- [Examples](./examples/concurrent.md) - See time-based patterns in action 
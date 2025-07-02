# Synchronization

Liten provides various synchronization primitives to coordinate between concurrent tasks. These primitives help you safely share data and coordinate work across multiple tasks.

## Mutex

A mutual exclusion lock that allows only one task to access shared data at a time.

### Basic Usage

```rust
use liten::{sync, task};

#[liten::main]
async fn main() {
    let mutex = sync::Mutex::new(0);
    
    let handles: Vec<_> = (0..10).map(|_| {
        let mutex = mutex.clone();
        task::spawn(async move {
            let mut guard = mutex.lock().await.unwrap();
            *guard += 1;
            // Guard is automatically released when dropped
        })
    }).collect();
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    let final_value = mutex.lock().await.unwrap();
    println!("Final value: {}", *final_value);
}
```

### Try Lock

Use `try_lock` when you don't want to wait:

```rust
use liten::sync;

#[liten::main]
async fn main() {
    let mutex = sync::Mutex::new("shared data");
    
    // Try to acquire the lock without waiting
    match mutex.try_lock() {
        Ok(guard) => {
            println!("Lock acquired: {}", *guard);
        }
        Err(_) => {
            println!("Lock is currently held by another task");
        }
    }
}
```

### Poisoning

Mutexes can be poisoned if a task panics while holding the lock:

```rust
use liten::sync;

#[liten::main]
async fn main() {
    let mutex = sync::Mutex::new(42);
    
    // Spawn a task that panics while holding the lock
    let handle = task::spawn({
        let mutex = mutex.clone();
        async move {
            let _guard = mutex.lock().await.unwrap();
            panic!("This will poison the mutex");
        }
    });
    
    // Wait for the panic
    let _ = handle.await;
    
    // Try to acquire the poisoned mutex
    match mutex.lock().await {
        Ok(_) => println!("Lock acquired successfully"),
        Err(_) => println!("Mutex is poisoned"),
    }
}
```

## Semaphore

A semaphore controls access to a limited resource by maintaining a count of available permits.

### Basic Usage

```rust
use liten::{sync, task, time};

#[liten::main]
async fn main() {
    // Allow only 3 concurrent accesses
    let semaphore = sync::Semaphore::new(3);
    
    let handles: Vec<_> = (0..10).map(|i| {
        let semaphore = semaphore.clone();
        task::spawn(async move {
            let _permit = semaphore.acquire().await;
            println!("Task {} acquired permit", i);
            
            // Simulate some work
            time::sleep(std::time::Duration::from_millis(100)).await;
            
            println!("Task {} releasing permit", i);
            // Permit is automatically released when dropped
        })
    }).collect();
    
    for handle in handles {
        handle.await.unwrap();
    }
}
```

### Try Acquire

Use `try_acquire` when you don't want to wait:

```rust
use liten::sync;

#[liten::main]
async fn main() {
    let semaphore = sync::Semaphore::new(1);
    
    // First acquire should succeed
    let permit1 = semaphore.try_acquire().unwrap();
    
    // Second acquire should fail
    match semaphore.try_acquire() {
        Some(_) => println!("Unexpectedly got a permit"),
        None => println!("No permits available"),
    }
    
    // Release the first permit
    drop(permit1);
    
    // Now we can acquire again
    let permit2 = semaphore.try_acquire().unwrap();
}
```

## Oneshot Channels

Oneshot channels allow sending a single value from one task to another.

### Basic Usage

```rust
use liten::{sync, task};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::oneshot::channel();
    
    // Spawn a task to send a value
    task::spawn(async move {
        sender.send("Hello from sender!").unwrap();
    });
    
    // Receive the value
    let message = receiver.await.unwrap();
    println!("Received: {}", message);
}
```

> oneshot channels is used internally in liten and has been by far the most versatile primitive in the sync module, for this use case.

### Error Handling

Handle cases where the sender or receiver is dropped:

```rust
use liten::{sync, task};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::oneshot::channel::<String>();
    
    // Drop the sender
    drop(sender);
    
    // Try to receive
    match receiver.await {
        Ok(value) => println!("Received: {}", value),
        Err(_) => println!("Sender was dropped"),
    }
}
```

### Try Receive

Use `try_recv` to check for a value without waiting:

```rust
use liten::sync;

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::oneshot::channel();
    
    // Try to receive before anything is sent
    match receiver.try_recv() {
        Ok(Some(value)) => println!("Received: {}", value),
        Ok(None) => println!("No value available yet"),
        Err(_) => println!("Channel is closed"),
    }
    
    // Send a value
    sender.send("Hello").unwrap();
    
    // Now try to receive
    match receiver.try_recv() {
        Ok(Some(value)) => println!("Received: {}", value),
        Ok(None) => println!("No value available yet"),
        Err(_) => println!("Channel is closed"),
    }
}
```

## Request-Response Pattern

The request-response pattern allows tasks to send requests and receive responses.

### Basic Usage

```rust
use liten::{sync, task};

#[liten::main]
async fn main() {
    let (requester, mut responder) = sync::request::channel();
    
    // Spawn a worker task
    let worker = task::spawn(async move {
        while let Some((request, response_sender)) = responder.recv().await {
            // Process the request
            let result = request * 2;
            
            // Send the response
            response_sender.send(result).unwrap();
        }
    });
    
    // Send requests
    let response1 = requester.send(5).await;
    let response2 = requester.send(10).await;
    
    println!("5 * 2 = {}", response1.unwrap());
    println!("10 * 2 = {}", response2.unwrap());
    
    // Stop the worker
    drop(requester);
    worker.await.unwrap();
}
```

### Multiple Workers

You can have multiple workers processing requests:

```rust
use liten::{sync, task};

#[liten::main]
async fn main() {
    let (requester, mut responder) = sync::request::channel();
    
    // Spawn multiple workers
    let workers: Vec<_> = (0..3).map(|worker_id| {
        let mut responder = responder.clone();
        task::spawn(async move {
            while let Some((request, response_sender)) = responder.recv().await {
                println!("Worker {} processing request: {}", worker_id, request);
                let result = request * request;
                response_sender.send(result).unwrap();
            }
        })
    }).collect();
    
    // Send requests
    let handles: Vec<_> = (1..=6).map(|i| {
        let requester = requester.clone();
        task::spawn(async move {
            requester.send(i).await.unwrap()
        })
    }).collect();
    
    // Collect results
    for handle in handles {
        let result = handle.await.unwrap();
        println!("Result: {}", result);
    }
    
    // Stop workers
    drop(requester);
    for worker in workers {
        worker.await.unwrap();
    }
}
```

## Pulse

A pulse is a simple synchronization primitive that can signal between tasks.

### Basic Usage

```rust
use liten::{sync, task, time};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::pulse();
    
    // Spawn a task that waits for the pulse
    let waiter = task::spawn(async move {
        println!("Waiting for pulse...");
        receiver.wait().unwrap();
        println!("Pulse received!");
    });
    
    // Wait a bit, then send the pulse
    time::sleep(std::time::Duration::from_millis(100)).await;
    sender.send().unwrap();
    
    waiter.await.unwrap();
}
```

### Multiple Pulses

You can send multiple pulses:

```rust
use liten::{sync, task, time};

#[liten::main]
async fn main() {
    let (sender, receiver) = sync::pulse();
    
    let counter = task::spawn(async move {
        let mut count = 0;
        loop {
            receiver.wait().unwrap();
            count += 1;
            println!("Pulse {} received", count);
            
            if count >= 3 {
                break;
            }
        }
    });
    
    // Send three pulses
    for i in 1..=3 {
        time::sleep(std::time::Duration::from_millis(50)).await;
        println!("Sending pulse {}", i);
        sender.send().unwrap();
    }
    
    counter.await.unwrap();
}
```

## Best Practices

### Choose the Right Primitive

- **Mutex**: When you need exclusive access to shared data
- **Semaphore**: When you need to limit concurrent access to a resource
- **Oneshot**: When you need to send a single value between tasks
- **Request-Response**: When you need bidirectional communication
- **Pulse**: When you need simple signaling between tasks

### Avoid Deadlocks

Be careful with lock ordering to avoid deadlocks:

```rust
// Good: Consistent lock ordering
let mutex1 = sync::Mutex::new(1);
let mutex2 = sync::Mutex::new(2);

let guard1 = mutex1.lock().await.unwrap();
let guard2 = mutex2.lock().await.unwrap();

// Bad: Inconsistent lock ordering can cause deadlocks
// let guard2 = mutex2.lock().await.unwrap();
// let guard1 = mutex1.lock().await.unwrap();
```

### Resource Cleanup

Always ensure resources are properly cleaned up:

```rust
use liten::sync;

#[liten::main]
async fn main() {
    let semaphore = sync::Semaphore::new(1);
    
    {
        let _permit = semaphore.acquire().await;
        // Do work with the permit
        println!("Working with permit");
        // Permit is automatically released when _permit goes out of scope
    }
    
    // Semaphore is available again
    let _permit2 = semaphore.acquire().await;
    println!("Got permit again");
}
```

### Performance Considerations

- **Mutex**: Use for short critical sections
- **Semaphore**: Good for limiting concurrency
- **Channels**: Efficient for communication between tasks
- **Pulse**: Lightweight for simple signaling

## Next Steps

Now that you understand synchronization, explore:
- [Time and Timers](./time.md) - Add time-based behavior to your synchronized code
- [Blocking Operations](./blocking.md) - Handle CPU-intensive work with synchronization
- [Examples](./examples/concurrent.md) - See synchronization in action 
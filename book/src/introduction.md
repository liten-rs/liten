# Introduction

**Liten** is a lightweight, high-performance async runtime for Rust designed with simplicity and efficiency in mind. It provides the essential building blocks for writing concurrent applications without the complexity of larger async runtimes.

## What is Liten?

Liten is a minimal async runtime that focuses on:

- **Simplicity**: Easy to understand and use
- **Performance**: Optimized for low-latency applications
- **Lightweight**: Small footprint with minimal dependencies
- **Composability**: Modular design that works well with other Rust libraries

## Key Features

### 🚀 **Task Management**
- Lightweight task spawning and scheduling
- Work-stealing scheduler for optimal performance
- Task cancellation and cleanup

### ⏰ **Time and Timers**
- Precise timer wheel implementation
- Sleep and timeout utilities
- Efficient timer management

### 🔒 **Synchronization Primitives**
- Mutex, Semaphore, and other sync primitives
- Oneshot channels for single-shot communication
- Request-response patterns

### 🧵 **Blocking Operations**
- Thread pool for CPU-intensive tasks
- File system operations
- Integration with blocking code

## When to Use Liten?

Liten is ideal for:

- **Embedded systems** where resource usage matters
- **High-performance servers** requiring low latency
- **Applications** that need fine-grained control over async behavior
- **Learning** async programming concepts in Rust

## Quick Example

```rust
use liten::{Runtime, task, time};

#[liten::main]
async fn main() {
    // Spawn a task
    let handle = task::spawn(async {
        time::sleep(std::time::Duration::from_secs(1)).await;
        "Hello from task!"
    });

    // Wait for the result
    let result = handle.await.unwrap();
    println!("{}", result);
}
```

## Getting Started

Ready to dive in? Check out the [Getting Started](./getting-started.md) guide to set up your first Liten project. 
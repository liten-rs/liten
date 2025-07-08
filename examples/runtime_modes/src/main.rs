use std::time::Duration;
use liten::{runtime::Runtime, task, time::sleep};

#[liten::main]
async fn main() {
    println!("=== Runtime Mode Examples ===\n");

    // Example 1: Single-threaded runtime
    println!("1. Single-threaded runtime:");
    Runtime::single_threaded()
        .block_on(async {
            println!("  Running in single-threaded mode");
            println!("  Worker thread count: {}", Runtime::single_threaded().worker_thread_count());
            println!("  Execution mode: {:?}", Runtime::single_threaded().execution_mode());
            
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

    println!();

    // Example 2: Multi-threaded runtime with default settings
    println!("2. Multi-threaded runtime (default):");
    Runtime::multi_threaded()
        .block_on(async {
            println!("  Running in multi-threaded mode");
            println!("  Worker thread count: {}", Runtime::multi_threaded().worker_thread_count());
            println!("  Execution mode: {:?}", Runtime::multi_threaded().execution_mode());
            
            // Spawn multiple tasks that will run on different threads
            let handles: Vec<_> = (0..4)
                .map(|i| {
                    task::spawn(async move {
                        println!("    Task {}: Starting on thread {:?}", i, std::thread::current().id());
                        sleep(Duration::from_millis(100)).await;
                        println!("    Task {}: Completed on thread {:?}", i, std::thread::current().id());
                        i
                    })
                })
                .collect();

            let results: Vec<_> = futures::future::join_all(handles).await;
            println!("    All results: {:?}", results);
        });

    println!();

    // Example 3: Multi-threaded runtime with custom thread count
    println!("3. Multi-threaded runtime (2 threads):");
    Runtime::multi_threaded()
        .worker_threads(2)
        .block_on(async {
            println!("  Running in multi-threaded mode with 2 worker threads");
            println!("  Worker thread count: {}", Runtime::multi_threaded().worker_threads(2).worker_thread_count());
            
            let handles: Vec<_> = (0..3)
                .map(|i| {
                    task::spawn(async move {
                        println!("    Task {}: Starting on thread {:?}", i, std::thread::current().id());
                        sleep(Duration::from_millis(50)).await;
                        println!("    Task {}: Completed on thread {:?}", i, std::thread::current().id());
                        i * 10
                    })
                })
                .collect();

            let results: Vec<_> = futures::future::join_all(handles).await;
            println!("    Results: {:?}", results);
        });

    println!();

    // Example 4: Builder pattern with explicit mode switching
    println!("4. Builder pattern with mode switching:");
    let mut builder = Runtime::builder();
    println!("  Default mode: {:?}", builder.execution_mode());
    
    builder = builder.single_threaded_mode();
    println!("  After single_threaded_mode(): {:?}", builder.execution_mode());
    
    builder = builder.multi_threaded_mode();
    println!("  After multi_threaded_mode(): {:?}", builder.execution_mode());
    
    builder.block_on(async {
        println!("  Running with builder configuration");
        let handle = task::spawn(async {
            println!("    Spawned task running");
            sleep(Duration::from_millis(50)).await;
            println!("    Spawned task completed");
            "success"
        });
        
        let result = handle.await;
        println!("    Task result: {}", result);
    });

    println!();

    // Example 5: Work stealing configuration
    println!("5. Multi-threaded runtime without work stealing:");
    Runtime::multi_threaded()
        .disable_work_stealing()
        .worker_threads(2)
        .block_on(async {
            println!("  Running with work stealing disabled");
            println!("  Worker thread count: {}", Runtime::multi_threaded().disable_work_stealing().worker_threads(2).worker_thread_count());
            
            let handle = task::spawn(async {
                println!("    Task running without work stealing");
                sleep(Duration::from_millis(50)).await;
                println!("    Task completed");
                "no work stealing"
            });
            
            let result = handle.await;
            println!("    Result: {}", result);
        });

    println!();

    // Example 6: Direct scheduler usage (new API)
    println!("6. Direct scheduler usage:");
    
    // Using convenience methods
    println!("  Using convenience methods:");
    let result1 = Runtime::block_on_single_threaded(async {
        println!("    Running with single-threaded convenience method");
        task::spawn(async { "single-threaded" }).await
    });
    println!("    Result: {}", result1);

    let result2 = Runtime::block_on_multi_threaded(async {
        println!("    Running with multi-threaded convenience method");
        task::spawn(async { "multi-threaded" }).await
    });
    println!("    Result: {}", result2);

    println!("=== Examples completed ===");
} 
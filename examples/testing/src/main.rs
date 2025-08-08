use std::{
  io::{Read, Write},
  net::TcpStream,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
  thread,
  time::Instant,
};

fn main() {
  let server_addr = "127.0.0.1:8084"; // Replace with your server address
  let num_connections = 10000; // Number of connections to establish

  let start_time = Instant::now();
  let mut handles = vec![];

  let errors = Arc::new(AtomicUsize::new(0));

  for _ in 0..num_connections {
    let error = errors.clone();
    let handle = thread::spawn(move || {
      if let Ok(mut stream) = TcpStream::connect(server_addr) {
        // Send the message
        if let Err(e) = stream.write(&[1, 2, 3, 4]) {
          eprintln!("Failed to write to stream: {}", e);
          return;
        }

        // Read the response
        let mut buf = [0; 4];
        match stream.read(&mut buf) {
          Ok(n) => {
            let response = String::from_utf8_lossy(&buf[..n]);
          }
          Err(e) => {
            error.fetch_add(1, Ordering::Relaxed);
            eprintln!("Failed to read from stream: {}", e)
          }
        }
      } else {
        error.fetch_add(1, Ordering::Relaxed);
        eprintln!("Failed to connect to server");
      }
    });

    handles.push(handle);
  }

  // Wait for all threads to complete
  for (inter, handle) in handles.into_iter().enumerate() {
    let _ = handle.join();
    // println!("inter: {inter}");
  }

  let duration = start_time.elapsed();
  println!("Time elapsed in benchmark: {:?}", duration);
  println!("{:?} errors", errors);
}

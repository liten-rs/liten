use std::time::Duration;

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
  let listener = TcpListener::bind("127.0.0.1:8084").await.unwrap();
  println!("Server listening on 127.0.0.1:8081");

  while let Ok((socket, _)) =
    tokio::time::timeout(Duration::from_secs(4), listener.accept())
      .await
      .unwrap()
  {
    // while let Ok((socket, _)) =
    // listener.accept().timeout(Duration::from_secs(4)).await.unwrap()

    tokio::task::spawn(async move {
      if let Err(e) = handle_connection(socket).await {
        println!("Error handling connection: {}", e);
      }
    });
  }
}

async fn handle_connection(
  mut socket: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
  // Read data from the socket
  let mut thing = vec![0, 0, 0, 0];
  let _n = socket.read_exact(&mut thing).await?;

  // Send a response back to the client
  let response = b"Hello, client!";
  socket.write_all(&Vec::from(response)).await?;

  Ok(())
}

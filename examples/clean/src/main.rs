use std::time::Duration;

use liten::{
  future::{FutureExt, Stream},
  io::net::socket::{TcpListener, TcpStream},
};

#[liten::main]
async fn main() {
  let listener = TcpListener::bind("127.0.0.1:8084").await.unwrap();
  println!("Server listening on 127.0.0.1:8081");

  while let Some(Ok((socket, _))) = listener.next().await {
    liten::task::spawn(async move {
      if let Err(e) = handle_connection(socket).await {
        println!("Error handling connection: {}", e);
      }
    });
  }
}

async fn handle_connection(
  socket: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
  // Read data from the socket
  let (n, buf) = socket.read(4).await?;

  // Send a response back to the client
  let response = vec![1, 2, 3, 4];
  socket.write(Vec::from(response)).await?;

  Ok(())
}

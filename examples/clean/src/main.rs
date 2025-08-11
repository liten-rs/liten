use std::time::Duration;

use liten::io::fs::File;
use liten::io::net::tcp::TcpStream;
use liten::io::{AsyncReadExt, AsyncWriteExt};

#[liten::main]
async fn main() {
  let file = File::open("./README.md").await.unwrap();

  let result = file.read_to_string().await.unwrap();

  println!("{result}");
  // let listener = TcpListener::bind("127.0.0.1:3002").await.unwrap();
  // println!("Server listening on 127.0.0.1:8081");
  //
  //
  // while let Some(Ok((socket, _))) = listener.next().await {
  //   liten::task::spawn(async move {
  //     println!("new");
  //     if let Err(e) = handle_connection(socket).await {
  //       println!("Error handling connection: {}", e);
  //     }
  //   });
  // }
}

async fn handle_connection(
  mut socket: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
  // Read data from the socket
  let (_n, _buf) = socket.read_all(Vec::from([0, 0, 0, 0])).await;
  _n?;

  // Send a response back to the client
  let response = vec![1, 2, 3, 4];
  let (result, _buf) = socket.write_all(response).await;

  result?;

  Ok(())
}

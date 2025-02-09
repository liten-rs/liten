use std::{
  error::Error,
  io::{self, Read, Write},
};

//use futures_util::AsyncReadExt;
use liten::{/*io::AsyncReadExt,*/ net::TcpListener, task};
use tracing::Level;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
  let tcp = TcpListener::bind("localhost:9000")?;
  loop {
    println!("waiting for stream");
    let (mut stream, _) = tcp.accept().await.unwrap();
    task::spawn(async move {
      let mut vec = Vec::default();
      stream.read_to_end(&mut vec).unwrap();
      tracing::info!("data received: {}", String::from_utf8(vec).unwrap());
      stream.write_all(b"HTTP/1.1\n200 OK")?;
      stream.flush()?;
      stream.shutdown(std::net::Shutdown::Write);

      Ok::<(), io::Error>(())
    });
  }
}

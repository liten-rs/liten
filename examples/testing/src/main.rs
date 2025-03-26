use std::{
  error::Error,
  io::{Read, Write},
};

use liten::net::TcpListener;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  subscriber::set_global_default(fmt().with_max_level(Level::TRACE).finish())?;

  let tcp = TcpListener::bind("localhost:9001")?;

  loop {
    let (mut stream, _) = tcp.accept().await.unwrap();

    let mut buf = [0];
    stream.read(&mut buf).unwrap();

    println!("data: {:?}", buf);

    stream.write_all(b"HTTP/1.1\n200 OK")?;
    stream.flush()?;

    let _ = stream.shutdown(std::net::Shutdown::Write);
  }
}

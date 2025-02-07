use std::{error::Error, io, time::Duration};

use futures_util::{AsyncReadExt, AsyncWriteExt};
use liten::{net::TcpListener, task};
use tracing::Level;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
  let tcp = TcpListener::bind("localhost:9000")?;
  loop {
    let (mut stream, _) = tcp.accept().await.unwrap();
    task::spawn(async move {
      let thing = {
        let mut vec = Vec::default();
        stream.read_to_end(&mut vec).await.unwrap();
        tracing::info!("data received: {}", String::from_utf8(vec).unwrap());
        stream.write_all(b"nice").await.unwrap();
        stream.write_all(b"nice").await.unwrap();
        stream.flush().await.unwrap();
        stream.close().await.unwrap();
        Ok::<(), io::Error>(())
      };
      println!("nice event{:#?}", &thing);
      thing
    });
  }
}

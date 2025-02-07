use futures_util::{AsyncReadExt, AsyncWriteExt};
use liten::{net::TcpStream, sync::Mutex, task};
use std::{error::Error, sync::Arc};
use tracing::Level;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
  let mut stream = TcpStream::connect("localhost:9000")?.await?;

  println!("have stream");

  stream.write_all(b"teting").await?;
  println!("wrote");
  stream.flush().await?;
  stream.close().await?;
  println!("flushed");
  let mut vec = Vec::default();
  stream.read_to_end(&mut vec).await?;
  println!("read {:#?}", vec);
  let data1 = Arc::new(Mutex::new(0));
  let data2 = Arc::clone(&data1);
  let data3 = Arc::clone(&data1);
  let data4 = Arc::clone(&data1);
  let data5 = Arc::clone(&data1);

  task::spawn(async move {
    let mut lock = data4.lock().await;
    *lock += 1;
  })
  .await;
  task::spawn(async move {
    let mut lock = data3.lock().await;
    *lock += 1;
  });
  task::spawn(async move {
    let mut lock = data2.lock().await;
    *lock += 1;
  })
  .await;
  task::spawn(async move {
    let mut lock = data1.lock().await;
    *lock += 1;
  });

  println!("nice {:#?}", *data5.lock().await);

  Ok(())
}

//use futures_util::AsyncReadExt;
use liten::{
  net::TcpStream,
  sync::{oneshot, Mutex},
  task,
};
use std::{
  error::Error,
  io::{Read, Write},
  sync::Arc,
  time::Duration,
};
use tracing::Level;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
  let mut stream = TcpStream::connect("localhost:9000")?.await?;

  stream.write_all(b"teting\0")?;
  stream.flush()?;
  stream.shutdown(std::net::Shutdown::Write)?;
  let mut vec = Vec::default();
  tracing::trace!("starting read");
  stream.read_to_end(&mut vec)?;
  tracing::trace!("read {:#?}", vec);

  //let (send, read) = oneshot::channel();
  //
  //std::thread::spawn(move || {
  //  std::thread::sleep(Duration::from_millis(500));
  //
  //  send.send("nice").unwrap();
  //  println!("done")
  //});
  //
  //let value = read.await;
  //
  //println!("nice value {}", value);
  //let data1 = Arc::new(Mutex::new(0));
  //let data2 = Arc::clone(&data1);
  //let data3 = Arc::clone(&data1);
  //let data4 = Arc::clone(&data1);
  //let data5 = Arc::clone(&data1);
  //
  //task::spawn(async move {
  //  let mut lock = data4.lock().await;
  //  *lock += 1;
  //})
  //.await;
  //task::spawn(async move {
  //  let mut lock = data3.lock().await;
  //  *lock += 1;
  //});
  //task::spawn(async move {
  //  let mut lock = data2.lock().await;
  //  *lock += 1;
  //})
  //.await;
  //task::spawn(async move {
  //  let mut lock = data1.lock().await;
  //  *lock += 1;
  //});
  //
  //println!("nice {:#?}", *data5.lock().await);

  Ok(())
}

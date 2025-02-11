use std::{
  error::Error,
  sync::{Arc, Mutex},
};
use tracing::Level;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let sub = tracing_subscriber::fmt().with_max_level(Level::TRACE).finish();
  tracing::subscriber::set_global_default(sub)?;
  //let mut stream = TcpStream::connect("localhost:9000")?.await?;
  //
  //stream.write_all(b"teting\0")?;
  //stream.flush()?;
  //stream.shutdown(std::net::Shutdown::Write)?;
  //let mut vec = Vec::default();
  //tracing::trace!("starting read");
  //stream.read_to_end(&mut vec)?;
  //tracing::trace!("read {:#?}", vec);

  //let (send, read) = oneshot::channel();
  //
  //liten::task::spawn(async move {
  //  std::thread::sleep(Duration::from_millis(500));
  //  send.send("nice").unwrap();
  //  println!("done")
  //})
  //.await;
  //
  //let value = read.await.unwrap();
  //
  //println!("nice value {}", value);
  let data1 = Arc::new(Mutex::new(0));
  let data2 = Arc::clone(&data1);
  let data3 = Arc::clone(&data1);
  let data4 = Arc::clone(&data1);
  let data5 = Arc::clone(&data1);
  println!("nice");

  //task::spawn(async move {
  let mut lock = data4.lock().unwrap();
  *lock += 1;
  println!("nice");
  drop(lock);
  //})
  //.await;
  //task::spawn(async move {
  //  let mut lock = data3.lock().unwrap();
  //  *lock += 1;
  //})
  //.await;
  //task::spawn(async move {
  //  let mut lock = data2.lock().unwrap();
  //  *lock += 1;
  //})
  //.await;
  //task::spawn(async move {
  //  let mut lock = data1.lock().unwrap();
  //  *lock += 1;
  //})
  //.await;

  println!("nice");
  println!("nice {:#?}", *data5.lock().unwrap());
  println!("nice");

  Ok(())
}

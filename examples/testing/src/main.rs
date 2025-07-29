use std::error::Error;

// // use liten::net::TcpListener;
// use tracing::{subscriber, Level};
// use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
  let testing = 32;
  let pointer: *const i32 = &testing;

  let address = pointer as u64;

  let address = address as *const i32;
  Ok(())
  // subscriber::set_global_default(fmt().with_max_level(Level::TRACE).finish())?;
  // liten::runtime::Runtime::builder().block_on(async {
  //   let tcp = TcpListener::bind("localhost:9001")?;
  //
  //   loop {
  //     let (mut stream, _) = tcp.accept().await.unwrap();
  //
  //     let mut vec = Vec::new();
  //     stream.read_to_end(&mut vec).unwrap();
  //
  //     println!("data: {:?}", String::from_utf8(vec).unwrap());
  //
  //     stream.write_all(b"HTTP/1.1\n200 OK")?;
  //     stream.flush()?;
  //
  //     let _ = stream.shutdown(std::net::Shutdown::Write);
  //   }
  // })
}

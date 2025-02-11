use std::{
  error::Error,
  io::{self, Read, Write},
};

//use futures_util::AsyncReadExt;
use liten::{/*io::AsyncReadExt,*/ net::TcpListener, task};
use tracing::Level;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> Result<(), Box<dyn Error>> {
  let _profiler = dhat::Profiler::new_heap();

  liten::runtime::Runtime::new().block_on(async {
    tracing_subscriber::fmt().with_max_level(Level::TRACE).finish();
    tracing::subscriber::set_global_default(
      tracing_subscriber::fmt().finish(),
    )?;
    let tcp = TcpListener::bind("localhost:9000")?;
    //loop {
    println!("waiting for stream");
    let (mut stream, _) = tcp.accept().await.unwrap();
    println!("nice");
    let mut vec = Vec::default();
    stream.read_to_end(&mut vec).unwrap();
    tracing::info!("data received: {}", String::from_utf8(vec).unwrap());
    stream.write_all(b"HTTP/1.1\n200 OK")?;
    stream.flush()?;
    let _ = stream.shutdown(std::net::Shutdown::Write);

    println!("done");

    Ok::<(), Box<dyn Error>>(())
    //}
  })
}

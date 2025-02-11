use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  //tracing_subscriber::fmt().with_max_level(Level::TRACE).finish();
  //tracing::subscriber::set_global_default(tracing_subscriber::fmt().finish())?;
  //
  //let tcp = TcpListener::bind("localhost:9000").await?;
  //
  //loop {
  //  let (mut stream, _) = tcp.accept().await.unwrap();
  //  task::spawn(async move {
  //    let mut vec = Vec::default();
  //    stream.read_to_end(&mut vec).await.unwrap();
  //    tracing::info!("data received: {}", String::from_utf8(vec).unwrap());
  //    stream.write_all(b"HTTP/1.1\n200 OK").await?;
  //    stream.flush().await?;
  //    let _ = stream.shutdown().await;
  //    Ok::<(), std::io::Error>(())
  //  });
  //}
  Ok(())
}

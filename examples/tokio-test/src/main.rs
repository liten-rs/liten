use std::error::Error;

use liten::sync::oneshot::sync_channel;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let sub = tracing_subscriber::FmtSubscriber::builder()
    .with_max_level(Level::TRACE)
    .finish();

  let _ = tracing::subscriber::set_global_default(sub);
  let (sender, receiver) = sync_channel::<u8>();
  let handler1 = tokio::task::spawn(async {
    sender.send(0).await.unwrap();
  });

  let handler2 = tokio::task::spawn(async {
    let result = receiver.await;
    assert_eq!(&result, &Ok(0));

    return result.unwrap();
  });

  handler1.await.unwrap();
  let result = handler2.await.unwrap();

  assert_eq!(result, 0);
  Ok(())
}

use std::time::Duration;

use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_driver() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

  lio::init();
  std::thread::sleep(Duration::from_millis(100));
  lio::shutdown().await;
}

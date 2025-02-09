use crate::sync::oneshot;
use std::time::Duration;

pub async fn sleep(duration: Duration) {
  let (sender, receiver) = oneshot::channel();
  std::thread::spawn(move || {
    std::thread::sleep(duration);
    sender.send(()).unwrap(); // in this scenario: oneshot errors if panics
  });

  receiver.await.unwrap(); // in this scenario: oneshot errors if panics
}

#[test]
fn testing() {
  crate::runtime::Runtime::new().block_on(async move {
    sleep(Duration::from_millis(200)).await;
    sleep(Duration::from_millis(0)).await;
  })
}

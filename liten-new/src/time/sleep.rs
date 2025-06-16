use crate::sync::oneshot;
use std::time::Duration;

// BAD CODE
pub async fn sleep(duration: Duration) {
  let (sender, receiver) = oneshot::channel();
  std::thread::spawn(move || {
    std::thread::sleep(duration);
    sender.send(()).unwrap(); // in this scenario: oneshot errors if panics
  });

  receiver.await.unwrap(); // in this scenario: oneshot errors if panics
}

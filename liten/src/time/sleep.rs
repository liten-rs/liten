use std::time::Duration;

pub async fn sleep(duration: Duration) {
  let (sender, receiver) = oneshot::channel();

  std::thread::spawn(move || {
    std::thread::sleep(duration);
    sender.send(()).unwrap()
  });

  receiver.await.unwrap()
}

#[test]
fn testing() {
  crate::runtime::Runtime::new().block_on(async move {
    sleep(Duration::from_millis(50)).await;
  })
}

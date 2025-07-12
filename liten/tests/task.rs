// Integration tests for the task module
use liten::task::spawn;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct TestFuture(Cell<u8>);
impl Future for TestFuture {
  type Output = u8;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u8> {
    if self.0.get() == 0 {
      self.0.set(1);
      cx.waker().wake_by_ref();
      Poll::Pending
    } else {
      Poll::Ready(99)
    }
  }
}

#[test]
fn task_poll_pending_then_ready_integration() {
  liten::runtime::Runtime::single_threaded().block_on(async {
    assert_eq!(spawn(TestFuture(Cell::new(0))).await.unwrap(), 99);
  })
}

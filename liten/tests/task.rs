#![cfg(all(feature = "runtime", feature = "sync"))]

// Integration tests for the task module
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use liten::sync::oneshot;

struct TestFuture(Cell<u8>);
impl Future for TestFuture {
  type Output = u8;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u8> {
    if self.0.get() == 0 {
      self.0.set(1);
      cx.waker().wake_by_ref();
      Poll::Pending
    } else {
      println!("very");
      Poll::Ready(99)
    }
  }
}

#[test]
fn task_poll_pending_then_ready_integration() {
  let runtime = liten::runtime::Runtime::single_threaded();

  let (sender, receiver) = oneshot::channel();

  let handle = runtime.spawn(async {
    let resutl = receiver.await.unwrap();
    println!("{resutl}");
  });

  let _ = sender.send(0);

  runtime.block_on(async {
    let ret = liten::task::spawn(async { TestFuture(Cell::new(0)).await });
    handle.await;
    // handle.cancel();
    // drop(handle);
    TestFuture(Cell::new(0)).await;

    ret.await;
  });
}

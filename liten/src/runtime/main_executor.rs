use std::{
  future::Future,
  sync::Arc,
  task::{Context as StdContext, Poll},
  thread,
};

use super::waker::RuntimeWaker;

pub struct GlobalExecutor;

impl GlobalExecutor {
  pub fn block_on<F, R>(f: F) -> R
  where
    F: Future<Output = R>,
  {
    let runtime_waker = Arc::new(RuntimeWaker::new(thread::current())).into();
    let mut context = StdContext::from_waker(&runtime_waker);
    let mut pinned = std::pin::pin!(f);

    loop {
      println!("main fut");
      match pinned.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => thread::park(),
      };
      println!("main fut");
    }
  }
}

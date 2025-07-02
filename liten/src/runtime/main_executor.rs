use std::{
  future::Future,
  task::{Context as StdContext, Poll},
};

use crate::loom::thread;

use super::waker::create_runtime_waker;

pub struct GlobalExecutor;

impl GlobalExecutor {
  pub fn block_on<F, R>(f: F) -> R
  where
    F: Future<Output = R>,
  {
    let runtime_waker = create_runtime_waker(thread::current());
    let mut context = StdContext::from_waker(&runtime_waker);
    let mut pinned = std::pin::pin!(f);

    loop {
      match pinned.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => {
          thread::park();
        }
      };
    }
  }
}

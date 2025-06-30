use std::{
  future::Future,
  task::{Context as StdContext, Poll},
};

use crate::loom::thread;

use super::{scheduler::worker::shared::Shared, waker::RuntimeWaker};

pub struct GlobalExecutor;

impl GlobalExecutor {
  pub fn block_on<F, R>(f: F, shared: &Shared) -> R
  where
    F: Future<Output = R>,
  {
    let runtime_waker =
      std::sync::Arc::new(RuntimeWaker::new(thread::current())).into();
    let mut context = StdContext::from_waker(&runtime_waker);
    let mut pinned = std::pin::pin!(f);

    loop {
      match pinned.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => {
          tracing::trace!("main parking");
          thread::park();
          tracing::trace!("main unparking");
        }
      };
    }
  }
}

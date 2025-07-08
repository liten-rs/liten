use std::{
  future::{Future, IntoFuture},
  task::{Context as StdContext, Poll},
};

use parking::Parker;

use crate::runtime::scheduler::waker::create_runtime_waker;

pub struct GlobalExecutor;

impl GlobalExecutor {
  pub fn block_on<F, R>(f: F) -> R
  where
    F: IntoFuture<Output = R>,
  {
    let parker = Parker::new();
    let mut pinned = std::pin::pin!(f.into_future());

    loop {
      let runtime_waker = create_runtime_waker(parker.unparker());
      let mut context = StdContext::from_waker(&runtime_waker);
      match pinned.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => {
          parker.park();
        }
      };
    }
  }
}

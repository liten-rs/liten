mod waker;
pub(crate) use waker::park_waker;

use std::{
  future::Future,
  task::{Context, Poll},
};

use parking::Parker;

pub fn block_on<Fut>(fut: Fut) -> Fut::Output
where
  Fut: Future,
{
  let parker = Parker::new();
  let mut pinned = std::pin::pin!(fut);

  loop {
    let runtime_waker = park_waker(parker.unparker());
    match pinned.as_mut().poll(&mut Context::from_waker(&runtime_waker)) {
      Poll::Ready(value) => return value,
      Poll::Pending => {
        parker.park();
      }
    };
  }
}

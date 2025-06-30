use pool::{BlockingPool, Job};

use crate::sync::oneshot;

mod pool;

pub async fn unblock<T, R>(f: T) -> R
where
  T: FnOnce() -> R + Send + 'static,
  R: 'static + Send,
{
  let (sender, receiver) = oneshot::channel::<R>();

  BlockingPool::get().insert(Job::new(sender, f));

  receiver.await.unwrap()
}

#[test]
fn blocking_testing() {
  crate::runtime::Runtime::builder().block_on(async move {
    assert!(unblock(|| 5).await == 5);
  })
}

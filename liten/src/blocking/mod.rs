pub(crate) mod pool;

use pool::{BlockingPool, Job};

use crate::sync::oneshot;

pub async fn unblock<T, R>(f: T) -> R
where
  T: FnOnce() -> R + Send + 'static,
  R: 'static + Send,
{
  let (sender, receiver) = oneshot::channel::<R>();

  BlockingPool::get().insert(Job::new(sender, f));

  receiver.await.unwrap()
}

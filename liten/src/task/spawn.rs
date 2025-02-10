use std::{
  future::{Future, IntoFuture},
  pin::Pin,
};

use crate::sync::oneshot;

use super::builder;

pub fn spawn<F>(fut: F) -> TaskHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send,
{
  builder().build(fut)
}

pub struct TaskHandle<Out>(pub(super) oneshot::Receiver<Out>);

impl<Out> IntoFuture for TaskHandle<Out>
where
  Out: 'static,
{
  type Output = Out;
  type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;
  fn into_future(self) -> Self::IntoFuture {
    Box::pin(async move { self.0.await.unwrap() })
  }
}

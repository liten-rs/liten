use std::{future::Future, pin::Pin};

pin_project_lite::pin_project! {
  pub struct TaskHandle<Out> {
    p: async_task::Task<Out>
  }
}

impl<O> TaskHandle<O> {
  pub(super) fn new(t: async_task::Task<O>) -> Self {
    TaskHandle { p: t }
  }

  pub fn join(self) -> O {
    crate::future::block_on(self)
  }
}

impl<Out> Future for TaskHandle<Out> {
  type Output = Out;
  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    let mut this = self.project();
    let pinned = Pin::new(&mut this.p);
    pinned.poll(cx)
  }
}

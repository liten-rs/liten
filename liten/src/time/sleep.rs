use super::{clock::TimerId, TimeDriver};
use std::{future::Future, time::Duration};

pub fn sleep(duration: Duration) -> Sleep {
  let duration_millis = duration.as_millis() as usize;

  let driver = TimeDriver::get();

  let timer_id = driver.insert(duration_millis);

  Sleep(timer_id)
}

pub struct Sleep(TimerId);

impl Future for Sleep {
  type Output = ();

  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    TimeDriver::get().poll(cx, self.0)
  }
}

#[crate::internal_test]
fn sleep_test() {
  crate::runtime::Runtime::single_threaded().block_on(async {
    sleep(Duration::from_millis(0)).await;
  })
}

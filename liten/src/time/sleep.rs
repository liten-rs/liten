use super::{clock::TimerId, TimeDriver};
use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
  time::Duration,
};

pub fn sleep(duration: Duration) -> Sleep {
  let duration_millis = duration.as_millis() as usize;

  let driver = TimeDriver::get();

  let timer_id = driver.insert(duration_millis);

  Sleep(timer_id)
}

pub fn interval(duration: Duration) -> Interval {
  Interval { interval_ms: duration.as_millis() as usize }
}

pub struct Interval {
  interval_ms: usize,
}

impl Interval {
  pub fn tick(&self) -> IntervalFut {
    IntervalFut {
      current_sleep: Sleep(TimeDriver::get().insert(self.interval_ms)),
    }
  }
}

pin_project_lite::pin_project! {
  pub struct IntervalFut {
        #[pin]
    current_sleep: Sleep,
  }
}

impl Future for IntervalFut {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.project();

    this.current_sleep.poll(cx)
  }
}

pub struct Sleep(TimerId);

impl Future for Sleep {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    TimeDriver::get().poll(cx, self.0)
  }
}

#[cfg(test)]
mod tests {
  use std::{
    future::pending,
    time::{Duration, Instant},
  };

  use crate::{
    future::{timeout::Timeout, FutureExt},
    time::{interval, sleep},
  };

  #[crate::internal_test]
  fn sleep_test() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      let fut1 = pending::<Result<(), Timeout>>();
      let instant = Instant::now();
      let result = fut1
        .or::<_, Result<(), Timeout>>(
          sleep(Duration::from_millis(10)).map(|_| Err(Timeout)),
        )
        .await;
      let instant2 = Instant::now();

      assert!(result == Err(Timeout));
      assert!((instant2 - instant).as_millis() > 10);
    })
  }

  #[crate::internal_test]
  fn interval_test() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      let inter = interval(Duration::from_millis(0));

      inter.tick().await;
      inter.tick().await;
    })
  }
}

use super::{clock::TimerId, TimeHandle};
use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
  time::Duration,
};

pub fn sleep(duration: Duration) -> Sleep {
  Sleep { duration: Some(duration), timer_id: None }
}

// pub fn interval(duration: Duration) -> Interval {
//   Interval { interval_ms: duration.as_millis() as usize }
// }
//
// pub struct Interval {
//   interval_ms: usize,
// }
//
// impl Interval {
//   pub fn tick(&self) -> IntervalFut {
//     IntervalFut {
//       current_sleep: Sleep(TimeDriver::get().insert(self.interval_ms)),
//     }
//   }
// }
//
// pin_project_lite::pin_project! {
//   pub struct IntervalFut {
//         #[pin]
//     current_sleep: Sleep,
//   }
// }
//
// impl Future for IntervalFut {
//   type Output = ();
//
//   fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//     let this = self.project();
//
//     this.current_sleep.poll(cx)
//   }
// }

pub struct Sleep {
  duration: Option<Duration>,
  timer_id: Option<TimerId>,
}

impl Future for Sleep {
  type Output = ();

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let Some(timer_id) = self.timer_id else {
      // Case 0: Timer Id doesn't exist
      self.timer_id = Some(TimeHandle::add_waker(
        cx.waker().clone(),
        self.duration.take().expect("Polled after Poll::Ready(..)").as_millis()
          as usize,
      ));

      return Poll::Pending;
    };

    assert!(self.duration.is_none(), "liten: logic error");

    if TimeHandle::entry_exists(&timer_id) {
      // Case 1: Timer hasn't finished.
      TimeHandle::update_timer_waker(timer_id, cx.waker().clone());
      Poll::Pending
    } else {
      // Case 2: Timer has finished.
      Poll::Ready(())
    }
  }
}

// #[cfg(test)]
// mod tests {
//   use crate::time::interval;
//   use std::time::Duration;
//
//   #[crate::internal_test]
//   fn interval_test() {
//     crate::runtime::Runtime::single_threaded().block_on(async {
//       let inter = interval(Duration::from_millis(0));
//
//       inter.tick().await;
//       inter.tick().await;
//     })
//   }
// }
#[cfg(all(test, not(loom)))]
mod tests2 {
  use std::{
    future::pending,
    time::{Duration, Instant},
  };

  use crate::{
    // future::{timeout::Timeout, FutureExt},
    time::sleep::sleep,
  };

  #[test]
  #[cfg(all(feature = "time", feature = "runtime"))]
  #[ignore] // Hangs because it runs in so many combinations.
  fn sleep_test() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      use crate::time::TimeHandle;

      for _ in 0..10 {
        let now = Instant::now();
        let _ = sleep(Duration::from_millis(10)).await;
        let elapsed = now.elapsed().as_millis();
        assert!(
          elapsed >= 10 && elapsed <= 11,
          "Something happened with elapsed: {elapsed}"
        );
      }

      // for try_ in 0..5 {
      let now = Instant::now();
      let _ = sleep(Duration::from_millis(500)).await;
      let elapsed = now.elapsed().as_millis();
      assert!(
        elapsed >= 500 && elapsed <= 505,
        "Something happened with elapsed: {elapsed}"
      );
      println!("ending");
      // // }
      // // println!("done");

      TimeHandle::shutdown();
    })
  }
}

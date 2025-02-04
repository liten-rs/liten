use std::{
  error::Error,
  future::Future,
  pin::Pin,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  task::{Context, Poll},
  thread,
  time::{Duration, Instant},
};

use liten::task;

pub struct Sleep {
  deadline: Instant,
  // Ensure we only spawn one sleeper thread.
  waker_registered: Arc<AtomicBool>,
}

impl Sleep {
  pub fn new(duration: Duration) -> Self {
    Self {
      deadline: Instant::now() + duration,
      waker_registered: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl Future for Sleep {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
    if Instant::now() >= self.deadline {
      return Poll::Ready(());
    }

    // Spawn a thread to sleep and wake the task if we haven't already.
    if !self.waker_registered.swap(true, Ordering::SeqCst) {
      let waker = cx.waker().clone();
      let deadline = self.deadline;
      thread::spawn(move || {
        let now = Instant::now();
        if deadline > now {
          thread::sleep(deadline - now);
        }
        waker.wake();
      });
    }

    Poll::Pending
  }
}

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  task::spawn(async move {
    println!("1st handler");
  });
  let handle_2 = task::spawn(async move { "from the await" });

  println!("2st handler {}", handle_2.await);

  println!("3: sync print");
  Ok(())
}

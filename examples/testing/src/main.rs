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

use futures_util::AsyncReadExt;
use liten::{net::TcpListener, task};

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
      task::spawn(async move {
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
  let tcp = TcpListener::bind("0.0.0.0:9000").unwrap();
  loop {
    println!("waiting");
    let (mut stream, _) = tcp.accept().await.unwrap();
    liten::task::spawn(async move {
      let mut vec = Vec::default();
      stream.read_to_end(&mut vec).await.unwrap();
      println!("data received: {:?}", String::from_utf8(vec).unwrap());
    });
  }
}

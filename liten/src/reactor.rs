use std::{
  collections::{hash_map::Entry, HashMap},
  io,
  sync::{Mutex, OnceLock},
  task::{Context, Poll, Waker},
};

use mio::{Interest, Token};

pub struct Reactor {
  registry: mio::Registry,
  statuses: Mutex<HashMap<Token, Status>>,
}

enum Status {
  Waker(Waker),
  Happened,
}

impl Reactor {
  pub fn register<S: mio::event::Source>(
    &self,
    source: &mut S,
    token: Token,
    interest: Interest,
  ) {
    self.registry.register(source, token, interest).unwrap();
  }

  pub fn deregister<S: mio::event::Source>(&self, source: &mut S) {
    self.registry.deregister(source).unwrap();
  }

  pub fn get() -> &'static Self {
    static REACTOR: OnceLock<Reactor> = OnceLock::new();

    REACTOR.get_or_init(|| {
      let poll = mio::Poll::new().unwrap();
      let reactor = Reactor {
        registry: poll.registry().try_clone().unwrap(),
        statuses: Mutex::new(HashMap::new()),
      };

      std::thread::Builder::new()
        .name("reactor".to_owned())
        .spawn(|| Reactor::run(poll))
        .unwrap();

      reactor
    })
  }

  fn run(mut poll: mio::Poll) {
    let reactor = Reactor::get();
    let mut events = mio::Events::with_capacity(1024);
    loop {
      poll.poll(&mut events, None).unwrap();

      for event in &events {
        println!("yes");
        let mut guard = reactor.statuses.lock().unwrap();
        println!("yes");

        let previous = guard.insert(event.token(), Status::Happened);
        if let Some(Status::Waker(waker)) = previous {
          waker.wake()
        }
      }
    }
  }
  pub fn poll(&self, token: Token, cx: &mut Context) -> Poll<io::Result<()>> {
    let mut guard = self.statuses.lock().unwrap();
    println!("nice");

    match guard.entry(token) {
      Entry::Vacant(vacant) => {
        vacant.insert(Status::Waker(cx.waker().clone()));
        Poll::Pending
      }
      Entry::Occupied(mut occupied) => {
        match occupied.get() {
          Status::Waker(waker) => {
            // skip clone is wakers are the same
            if dbg!(!waker.will_wake(cx.waker())) {
              println!("hm");
              occupied.insert(Status::Waker(cx.waker().clone()));
            }
            Poll::Pending
          }
          Status::Happened => {
            occupied.remove();
            Poll::Ready(Ok(()))
          }
        }
      }
    }
  }
}

mod registration;

pub use registration::IoRegistration;

use std::{
  collections::{hash_map::Entry, HashMap},
  io,
  sync::{Arc, LazyLock, Mutex, OnceLock},
  task::{Context, Poll, Waker},
  thread,
};

use mio::{event::Source, Interest, Token};

use crate::context;

pub struct IOEventLoop {
  registry: mio::Registry,
  statuses: Mutex<HashMap<Token, Status>>,
}

enum Status {
  Waker(Waker),
  Happened,
}

impl std::fmt::Debug for Status {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Status::");

    match self {
      Status::Happened => f.write_str("Happened"),
      Status::Waker(_) => f.write_str("Waker(...)"),
    }
  }
}

static IO_EVENT_LOOP: LazyLock<IOEventLoop> = LazyLock::new(|| {
  // Only gets run once. on first access
  let poll = mio::Poll::new().unwrap();
  let event_loop = IOEventLoop {
    registry: poll.registry().try_clone().unwrap(),
    statuses: Mutex::new(HashMap::new()),
  };

  thread::Builder::new()
    .name("liten-io".to_owned())
    .spawn(|| IOEventLoop::run(poll))
    .unwrap();

  event_loop
});

impl IOEventLoop {
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
    &IO_EVENT_LOOP
  }

  fn run(mut poll: mio::Poll) {
    let reactor = IOEventLoop::get();
    let mut events = mio::Events::with_capacity(1024);
    loop {
      poll.poll(&mut events, None).unwrap();

      for event in &events {
        let mut guard = reactor.statuses.lock().unwrap();

        if let Some(Status::Waker(waker)) =
          guard.insert(event.token(), Status::Happened)
        {
          waker.wake()
        }
      }
    }
  }

  /// Polls on specified token
  ///
  /// If token doesn't exist in the registry:
  ///   Token gets inserted with its waker.
  /// If it does:
  ///   If Status::Happened: Gets removed and polls ready
  ///   if Status::Waker(waker): inserts with waker.
  pub fn poll(&self, token: Token, cx: &mut Context) -> Poll<io::Result<()>> {
    let mut guard = self.statuses.lock().unwrap();

    match guard.entry(token) {
      Entry::Vacant(vacant) => {
        vacant.insert(Status::Waker(cx.waker().clone()));
        Poll::Pending
      }
      Entry::Occupied(mut occupied) => {
        match occupied.get() {
          Status::Waker(waker) => {
            // skip clone is wakers are the same
            if !waker.will_wake(cx.waker()) {
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

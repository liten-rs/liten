mod registration;

pub use registration::IoRegistration;

use std::{
  collections::{hash_map::Entry, HashMap},
  io,
  sync::{Arc, LazyLock, Mutex},
  task::{Context, Poll, Waker},
  thread,
};

use mio::{Interest, Token};

use crate::context;

#[derive(Debug)]
pub struct IOEventLoop {
  registry: mio::Registry,
  statuses: Mutex<HashMap<Token, Waker>>,
}

static IO_EVENT_LOOP_STARTED: LazyLock<Arc<Mutex<bool>>> =
  LazyLock::new(|| Arc::new(Mutex::new(false)));

impl IOEventLoop {
  pub(crate) fn init() -> IOEventLoop {
    let mut lock = IO_EVENT_LOOP_STARTED.lock().unwrap();
    if *lock {
      // This is such a bad developer error so this shouldn't happen.
      panic!("internal 'liten' error: started io-event loop more times than 1.")
    }

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

    *lock = true;

    event_loop
  }
  pub fn register<S: mio::event::Source>(
    &self,
    source: &mut S,
    token: Token,
    interest: Interest,
  ) -> io::Result<()> {
    self.registry.register(source, token, interest)
  }

  pub fn deregister<S: mio::event::Source>(
    &self,
    source: &mut S,
  ) -> io::Result<()> {
    self.registry.deregister(source)
  }

  fn run(mut poll: mio::Poll) {
    let reactor = context::get_context().io();
    let mut events = mio::Events::with_capacity(1024);
    loop {
      poll.poll(&mut events, None).unwrap();

      for event in &events {
        let mut guard = reactor.statuses.lock().unwrap();

        if let Some(waker) = guard.remove(&event.token()) {
          waker.wake()
        }
      }
    }
  }

  /// Polls on specified token
  ///
  /// If token doesn't exist in the registry:
  ///   Token gets inserted with its waker.
  ///
  /// # Outputs:
  /// ## Waker from cx hasn't been registered before:
  /// Registeres it and returns [Poll::Pending]
  ///
  /// ## Future waker has been registered:
  ///
  /// If event hasn't happened yet: return [Poll::Pending]
  ///
  /// If event has happened: remove entry and return [Poll::Ready]
  pub fn poll(&self, token: Token, cx: &mut Context) -> Poll<()> {
    let mut guard = self.statuses.lock().unwrap();

    match guard.entry(token) {
      Entry::Vacant(vacant) => {
        vacant.insert(cx.waker().clone());
        Poll::Pending
      }
      Entry::Occupied(mut occupied) => {
        if !occupied.get().will_wake(cx.waker()) {
          occupied.insert(cx.waker().clone());
        }
        Poll::Pending
      }
    }
  }
}

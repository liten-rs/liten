mod event_token;
mod registration;

use event_token::EventToken;
pub use registration::EventRegistration;

use std::{
  collections::{hash_map::Entry, HashMap},
  io,
  sync::{Arc, Mutex},
  task::{Context, Waker},
};

use mio::{Events, Interest, Token};

/// IO-Driver
// TODO: implement reference counting for Handle, so that when driver is shutting down, it only
// allows when there is no handlers left.
#[derive(Debug)]
pub struct Driver {
  poll: mio::Poll,
  /// Is only used when consumers are calling [`Self::handle()`]
  handle_to_give: Handle,
}

impl Driver {
  pub fn new() -> io::Result<Driver> {
    let poll = mio::Poll::new()?;
    let handle_to_give = Handle {
      registry: poll.registry().try_clone()?,
      event_token: Arc::new(EventToken::new()),
      wakers: Arc::new(Mutex::new(HashMap::new())),
    };

    Ok(Driver { handle_to_give, poll })
  }

  pub fn handle(&self) -> Handle {
    self.handle_to_give.clone()
  }

  pub fn turn(&mut self) -> bool {
    // FIXME: This doesn't quit.
    let mut events = Events::with_capacity(1024);
    self.poll.poll(&mut events, None).unwrap();

    for event in &events {
      if event.token() == EventToken::SHUTDOWN_SIGNAL_TOKEN {
        return true; // Wakeup-call
      };
      let mut guard =
        self.handle_to_give.wakers.lock().expect("wakers lock poisoned");
      if let Some(waker) = guard.remove(&event.token()) {
        waker.wake()
      }
    }
    false
  }
}

/// Distributable handle to event loop driver. Handle can register and unregister events that the
/// driver should wake futures to.
#[derive(Debug)]
pub struct Handle {
  registry: mio::Registry,
  event_token: Arc<EventToken>,
  wakers: Arc<Mutex<HashMap<Token, Waker>>>,
}

impl Clone for Handle {
  fn clone(&self) -> Self {
    Handle {
      wakers: self.wakers.clone(),
      event_token: self.event_token.clone(),
      registry: self.registry.try_clone().expect(
        "Couldn't clone mio::Registry for cloning of liten::events::Handle",
      ),
    }
  }
}

impl Handle {
  pub fn next_token(&self) -> Token {
    self.event_token.token()
  }
  pub fn shutdown_waker(&self) -> mio::Waker {
    mio::Waker::new(&self.registry, EventToken::SHUTDOWN_SIGNAL_TOKEN).unwrap()
  }
  pub(self) fn register(
    &self,
    source: &mut dyn mio::event::Source,
    token: Token,
    interest: Interest,
  ) -> io::Result<()> {
    self.registry.register(source, token, interest)
  }

  pub(self) fn reregister(
    &self,
    source: &mut dyn mio::event::Source,
    token: Token,
    interest: Interest,
  ) -> io::Result<()> {
    self.registry.reregister(source, token, interest)
  }

  pub(self) fn deregister(
    &self,
    source: &mut dyn mio::event::Source,
  ) -> io::Result<()> {
    self.registry.deregister(source)
  }

  /// Registers a waker for io-bound futures that are pending.
  ///
  /// If token doesn't exist in the registry:
  ///   Token gets inserted with its waker.
  pub fn poll(&self, token: Token, cx: &mut Context) {
    let mut guard = self.wakers.lock().unwrap();

    match guard.entry(token) {
      Entry::Vacant(vacant) => {
        vacant.insert(cx.waker().clone());
      }
      Entry::Occupied(mut occupied) => {
        tracing::warn!(token = ?token, "entry occupied");
        let waker = cx.waker().clone();
        if !occupied.get().will_wake(&waker) {
          occupied.insert(waker);
        }
      }
    }
  }
}

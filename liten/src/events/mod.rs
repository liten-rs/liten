mod registration;

pub use registration::EventRegistration;

use std::{
  collections::{hash_map::Entry, HashMap},
  io,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
  },
  task::{Context, Waker},
};

use mio::{Events, Interest, Token};

struct TokenState(AtomicUsize);

const WAKEUP_TOKEN: Token = Token(0);

impl TokenState {
  pub fn new() -> TokenState {
    TokenState(AtomicUsize::new(1)) // 0 is specialcase
  }
  pub fn next_token(&self) -> Token {
    debug_assert!(
      self.0.load(Ordering::Relaxed) != 0,
      "can't call next_token on wakeup token"
    );
    Token(self.0.fetch_add(1, Ordering::Acquire))
  }
}

/// IO-Driver
#[derive(Debug)]
pub struct Driver {
  poll: mio::Poll,
}

/// Reference to the IO driver
pub struct Handle {
  registry: mio::Registry,
  // Using a stdMutex because events::Handle is not in a async context and doesn't fit a async
  // model.
  wakers: Mutex<HashMap<Token, Waker>>,

  token_state: TokenState,
}

impl Handle {
  pub fn next_token(&self) -> Token {
    self.token_state.next_token()
  }
  pub fn mio_waker(&self) -> mio::Waker {
    mio::Waker::new(&self.registry, WAKEUP_TOKEN).unwrap()
  }
  pub fn from_driver_ref(driver: &Driver) -> io::Result<Self> {
    Ok(Self {
      registry: driver.poll.registry().try_clone()?,
      wakers: Mutex::new(HashMap::new()),
      token_state: TokenState::new(),
    })
  }
  pub fn register(
    &self,
    source: &mut dyn mio::event::Source,
    token: Token,
    interest: Interest,
  ) -> io::Result<()> {
    self.registry.register(source, token, interest)
  }

  pub fn reregister(
    &self,
    source: &mut dyn mio::event::Source,
    token: Token,
    interest: Interest,
  ) -> io::Result<()> {
    self.registry.reregister(source, token, interest)
  }

  pub fn deregister(
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
        if !occupied.get().will_wake(cx.waker()) {
          occupied.insert(cx.waker().clone());
        }
      }
    }
  }
}

impl Driver {
  pub fn new() -> io::Result<(Driver, Handle)> {
    let driver = Driver { poll: mio::Poll::new().unwrap() };

    let handle = Handle::from_driver_ref(&driver)?;

    Ok((driver, handle))
  }

  pub fn turn(&mut self, handle: &Handle) -> bool {
    // FIXME: This doesn't quit.
    let mut events = Events::with_capacity(1024);
    self.poll.poll(&mut events, None).unwrap();

    for event in &events {
      match event.token() {
        WAKEUP_TOKEN => return true, // Wakeup-call
        _ => {
          let mut guard = handle.wakers.lock().unwrap();
          if let Some(waker) = guard.remove(&event.token()) {
            waker.wake()
          }
        }
      }
    }
    false
  }
}

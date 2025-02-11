mod registration;

pub use registration::IoRegistration;

use std::{
  collections::{hash_map::Entry, HashMap},
  io,
  sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, LazyLock, Mutex,
  },
  task::{Context, Poll, Waker},
  thread,
};

use mio::{Events, Interest, Token};

use crate::sync::oneshot;

#[derive(Debug)]
pub struct TokenGenerator(AtomicUsize);

impl TokenGenerator {
  pub fn new_wakeup() -> TokenGenerator {
    TokenGenerator(AtomicUsize::new(0))
  }
  pub fn new() -> TokenGenerator {
    tracing::trace!("this should only happen once io_loop_mod");
    TokenGenerator(AtomicUsize::new(1)) // 0 is specialcase
  }
  pub fn next_token(&self) -> Token {
    Token(self.0.fetch_add(1, Ordering::Acquire))
  }
}

impl Into<Token> for TokenGenerator {
  fn into(self) -> Token {
    Token(self.0.load(Ordering::Relaxed))
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
  wakers: Mutex<HashMap<Token, Waker>>,
  token_generator: TokenGenerator,
}

impl Handle {
  pub fn next_token(&self) -> Token {
    self.token_generator.next_token()
  }
  pub fn from_driver_ref(driver: &Driver) -> io::Result<Self> {
    Ok(Self {
      registry: driver.poll.registry().try_clone()?,
      wakers: Mutex::new(HashMap::new()),
      token_generator: TokenGenerator::new(),
    })
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
  //pub(crate) fn init() -> IODriver {
  //  if context::has_init() {
  //    // This is such a bad developer error so this shouldn't happen.
  //    panic!(
  //      "internal 'liten' error: started io-event loop more times than 1."
  //    );
  //  }
  //
  //  // Only gets run once. on first access
  //  let poll = mio::Poll::new().unwrap();
  //  let event_loop = IODriver {
  //    registry: poll.registry().try_clone().unwrap(),
  //    statuses: Mutex::new(HashMap::new()),
  //    token_generator: TokenGenerator::new(),
  //  };
  //
  //  thread::Builder::new()
  //    .name("liten-io".to_owned())
  //    .spawn(|| IODriver::run(poll))
  //    .unwrap();
  //
  //  event_loop
  //}

  pub fn turn(&mut self, handle: &Handle) -> bool {
    // FIXME: If it runs on another thread will this fuck up?
    let mut events = Events::with_capacity(1024);
    self.poll.poll(&mut events, None).unwrap();

    for event in &events {
      match event.token().0 {
        0 => return true, // Wakeup-call
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

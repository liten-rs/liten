use std::sync::atomic::{AtomicUsize, Ordering};

use mio::Token;

#[derive(Debug)]
pub(super) struct EventToken {
  current_token: AtomicUsize,
}

impl EventToken {
  pub const SHUTDOWN_SIGNAL_TOKEN: Token = Token(0);

  pub fn new() -> EventToken {
    EventToken { current_token: AtomicUsize::new(1) }
  }
  pub fn token(&self) -> Token {
    Token(self.current_token.fetch_add(1, Ordering::Acquire))
  }
}

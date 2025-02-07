use std::io;

use crate::context;
use mio::{event::Source, Interest, Token};

#[derive(Clone, Copy)]
pub struct IoRegistration {
  token: Token,
  interest: Interest,
}

impl IoRegistration {
  pub fn new(interest: Interest) -> IoRegistration {
    let token = context::get_context().next_registration_token();
    IoRegistration { token, interest }
  }

  pub fn register(&self, source: &mut impl Source) -> io::Result<()> {
    context::get_context().io().register(source, self.token, self.interest)
  }

  pub fn deregister(&self, source: &mut impl Source) -> io::Result<()> {
    context::get_context().io().deregister(source)
  }

  pub fn token(&self) -> Token {
    self.token
  }
}

use std::{
  io,
  task::{Context, Waker},
};

use crate::context;
use mio::{event::Source, Interest, Token};

#[derive(Clone, Copy)]
pub struct IoRegistration {
  token: Token,
  interest: Interest,
}

impl IoRegistration {
  pub fn new(interest: Interest) -> IoRegistration {
    let token = context::with_context(|ctx| ctx.next_registration_token());
    IoRegistration { token, interest }
  }

  pub fn register(&self, source: &mut impl Source) -> io::Result<()> {
    context::with_context(|ctx| {
      ctx.io().register(source, self.token, self.interest)
    })
  }

  pub fn deregister(&self, source: &mut impl Source) -> io::Result<()> {
    context::with_context(|ctx| ctx.io().deregister(source))
  }

  pub fn token(&self) -> Token {
    self.token
  }

  pub fn register_io_waker(&self, waker: &mut Context) {
    context::with_context(|ctx| ctx.io().poll(self.token(), waker));
  }
}

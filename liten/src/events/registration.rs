use std::{io, task::Context};

use crate::context;
use mio::{event::Source, Interest, Token};

#[derive(Clone, Copy)]
pub struct EventRegistration {
  token: Token,
  interest: Interest,
}

impl EventRegistration {
  #[allow(unused)]
  pub fn new(
    interest: Interest,
    source: &mut impl Source,
  ) -> io::Result<EventRegistration> {
    context::with_context(|ctx| {
      let token = ctx.handle().io().next_token();
      ctx.handle().io().register(source, token, interest)?;

      Ok(EventRegistration { token, interest })
    })
  }

  #[allow(unused)]
  pub fn token(&self) -> Token {
    self.token
  }

  #[allow(unused)]
  pub fn is_read(&self) -> bool {
    self.interest.is_readable()
  }

  #[allow(unused)]
  pub fn is_write(&self) -> bool {
    self.interest.is_writable()
  }

  #[allow(unused)]
  pub fn reregister(
    &mut self,
    source: &mut impl Source,
    interest: Interest,
  ) -> io::Result<()> {
    context::with_context(|ctx| {
      ctx.handle().io().reregister(source, self.token, interest)?;
      self.interest = interest;
      Ok(())
    })
  }

  #[allow(unused)]
  pub fn deregister(&self, source: &mut impl Source) -> io::Result<()> {
    context::with_context(|ctx| ctx.handle().io().deregister(source))
  }

  #[allow(unused)]
  pub fn associate_waker(&self, waker: &mut Context) {
    context::with_context(|ctx| ctx.handle().io().poll(self.token(), waker));
  }
}

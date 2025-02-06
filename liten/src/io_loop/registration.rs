use super::IOEventLoop;
use crate::context;
use mio::{event::Source, Interest, Token};

pub struct IoRegistration<S: Source> {
  source: S,
  token: Token,
}

impl<S> IoRegistration<S>
where
  S: Source,
{
  pub fn new(mut source: S, interest: Interest) -> IoRegistration<S> {
    let token = Token(context::get_context().task_id_inc());
    IOEventLoop::get().register(&mut source, token.clone(), interest);
    Self { source, token }
  }

  pub fn token(&self) -> Token {
    self.token
  }

  pub fn inner(&self) -> &S {
    &self.source
  }
}

//impl<S> Source for IoRegistration<S>
//where
//  S: Source,
//{
//  fn register(
//    &mut self,
//    registry: &mio::Registry,
//    token: Token,
//    interests: Interest,
//  ) -> io::Result<()> {
//    self.source.register(registry, token, interests)
//  }
//
//  fn reregister(
//    &mut self,
//    registry: &mio::Registry,
//    token: Token,
//    interests: Interest,
//  ) -> io::Result<()> {
//    self.source.reregister(registry, token, interests)
//  }
//
//  fn deregister(&mut self, registry: &mio::Registry) -> io::Result<()> {
//    self.source.deregister(registry)
//  }
//}

impl<S> Drop for IoRegistration<S>
where
  S: Source,
{
  fn drop(&mut self) {
    IOEventLoop::get().deregister(&mut self.source)
  }
}

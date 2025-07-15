use std::future::Future;

pub(crate) mod single_threaded;

pub use single_threaded::SingleThreaded;

pub trait Scheduler {
  fn block_on<F>(&self, fut: F) -> F::Output
  where
    F: Future;
}

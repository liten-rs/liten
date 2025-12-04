use crate::{OperationProgress, driver::OpStore, op::Operation};

#[cfg(linux)]
mod io_uring;
mod polling;
#[cfg(linux)]
pub use io_uring::*;
pub use polling::*;

pub trait IoBackend {
  // fn init() -> Self;
  fn tick(&self, store: &OpStore, can_wait: bool);
  fn submit<O>(&self, op: O, store: &OpStore) -> OperationProgress<O>
  where
    O: Operation + Sized;
  fn notify(&self);
}

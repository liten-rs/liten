pub(crate) mod single_threaded;

use async_task::Runnable;
pub use single_threaded::*;

pub trait Scheduler {
  fn tick(&self, runnables: impl Iterator<Item = Runnable>);
}

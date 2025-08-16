use crate::runtime::scheduler::Scheduler;

pub struct SingleThreaded;

impl Scheduler for SingleThreaded {
  fn tick(&self, runnables: impl Iterator<Item = async_task::Runnable>) {
    for runnable in runnables {
      runnable.run();
    }
  }
}

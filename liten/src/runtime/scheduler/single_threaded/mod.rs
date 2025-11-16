use crate::runtime::scheduler::Scheduler;

pub struct SingleThreaded;

impl Scheduler for SingleThreaded {
  fn schedule(&self, runnable: async_task::Runnable) {
    let _ = runnable.run();
  }
}

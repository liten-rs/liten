mod main_executor;
pub(crate) mod scheduler;
mod waker;

use scheduler::Scheduler;
use std::{future::Future, num::NonZero};

pub struct Runtime;

impl Runtime {
  pub fn builder() -> RuntimeBuilder {
    RuntimeBuilder::default()
  }

  fn with_config<F, Res>(fut: F, config: RuntimeBuilder) -> Res
  where
    F: Future<Output = Res>,
  {
    Scheduler.block_on(fut, config)
  }
}

#[derive(Default, Clone)]
#[non_exhaustive]
pub enum RuntimeThreads {
  #[default]
  Cpus,
  Number(NonZero<usize>),
}

impl RuntimeThreads {
  pub(crate) fn get_threads(&self) -> NonZero<usize> {
    match self {
      RuntimeThreads::Number(num) => *num,
      RuntimeThreads::Cpus => std::thread::available_parallelism().unwrap(),
    }
  }

  pub(crate) fn set_threads(&mut self, value: NonZero<usize>) {
    *self = RuntimeThreads::Number(value);
  }
}

#[derive(Clone)]
pub struct RuntimeBuilder {
  max_threads: RuntimeThreads,
  enable_work_stealing: bool,
}

impl Default for RuntimeBuilder {
  fn default() -> Self {
    Self { enable_work_stealing: true, max_threads: RuntimeThreads::default() }
  }
}

impl RuntimeBuilder {
  pub fn num_workers(mut self, num: usize) -> Self {
    self.max_threads.set_threads(NonZero::new(num).unwrap());
    self
  }

  pub fn disable_work_stealing(mut self) -> Self {
    self.enable_work_stealing = false;
    self
  }

  pub(crate) fn get_num_workers(&self) -> NonZero<usize> {
    self.max_threads.get_threads()
  }

  pub fn block_on<F, Res>(self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    Runtime::with_config(fut, self)
  }
}

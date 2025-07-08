use std::future::IntoFuture;

#[cfg(feature = "blocking")]
use crate::blocking::pool::BlockingPool;
use crate::runtime::scheduler::{
  multi_threaded::Multithreaded, single_threaded::SingleThreaded,
};
#[cfg(feature = "time")]
use crate::time::TimeDriver;

pub mod scheduler;

pub struct Runtime<S: scheduler::Scheduler> {
  scheduler: S,
}

impl Runtime<SingleThreaded> {
  pub fn single_threaded() -> Self {
    Runtime { scheduler: SingleThreaded }
  }
}
impl Runtime<Multithreaded> {
  pub fn multi_threaded() -> Self {
    Runtime { scheduler: Multithreaded::default() }
  }
}

impl<T> Runtime<T>
where
  T: scheduler::Scheduler,
{
  pub fn block_on<F, R>(self, fut: F) -> R
  where
    F: IntoFuture<Output = R>,
  {
    let to_return = self.scheduler.block_on(fut);

    #[cfg(feature = "time")]
    TimeDriver::shutdown();
    #[cfg(feature = "blocking")]
    BlockingPool::shutdown();

    to_return
  }
}

// mod main_executor;
// pub(crate) mod scheduler;
// mod waker;
//
// use scheduler::{Scheduler, SchedulerTrait, single_threaded::SingleThreadedScheduler, multi_threaded::MultiThreadedScheduler};
// use std::{future::Future, num::NonZero};
//
// /// A runtime that can execute async tasks.
// ///
// /// The runtime can be configured to run in either single-threaded or multi-threaded mode.
// /// Single-threaded mode is useful for simple applications or when you need deterministic
// /// execution, while multi-threaded mode provides better performance for CPU-bound tasks.
// pub struct Runtime;
//
// impl Runtime {
//   /// Creates a new runtime builder for configuring a runtime.
//   pub fn builder() -> RuntimeBuilder {
//     RuntimeBuilder::default()
//   }
//
//   /// Creates a single-threaded runtime with default settings.
//   pub fn single_threaded() -> RuntimeBuilder {
//     RuntimeBuilder::single_threaded()
//   }
//
//   /// Creates a multi-threaded runtime with default settings.
//   pub fn multi_threaded() -> RuntimeBuilder {
//     RuntimeBuilder::multi_threaded()
//   }
//
//   /// Blocks the current thread using a single-threaded scheduler.
//   ///
//   /// This is equivalent to `Runtime::single_threaded().block_on(fut)`.
//   pub fn block_on_single_threaded<F, Res>(fut: F) -> Res
//   where
//     F: Future<Output = Res>,
//   {
//     SingleThreadedScheduler.block_on(fut, RuntimeBuilder::single_threaded())
//   }
//
//   /// Blocks the current thread using a multi-threaded scheduler.
//   ///
//   /// This is equivalent to `Runtime::multi_threaded().block_on(fut)`.
//   pub fn block_on_multi_threaded<F, Res>(fut: F) -> Res
//   where
//     F: Future<Output = Res>,
//   {
//     MultiThreadedScheduler.block_on(fut, RuntimeBuilder::multi_threaded())
//   }
//
//   fn with_config<F, Res>(fut: F, config: RuntimeBuilder) -> Res
//   where
//     F: Future<Output = Res>,
//   {
//     Scheduler.block_on(fut, config)
//   }
// }
//
// /// Configuration for the number of worker threads in a multi-threaded runtime.
// #[derive(Default, Clone)]
// #[non_exhaustive]
// pub enum WorkerThreads {
//   /// Use the number of available CPU cores (default)
//   #[default]
//   Auto,
//   /// Use a specific number of threads
//   Number(NonZero<usize>),
// }
//
// impl WorkerThreads {
//   pub(crate) fn get_threads(&self) -> NonZero<usize> {
//     match self {
//       WorkerThreads::Number(num) => *num,
//       WorkerThreads::Auto => std::thread::available_parallelism().unwrap(),
//     }
//   }
//
//   pub(crate) fn set_threads(&mut self, value: NonZero<usize>) {
//     *self = WorkerThreads::Number(value);
//   }
// }
//
// /// The execution mode for the runtime.
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum ExecutionMode {
//   /// Single-threaded execution - all tasks run on the main thread
//   SingleThreaded,
//   /// Multi-threaded execution - tasks are distributed across worker threads
//   MultiThreaded,
// }
//
// /// Builder for configuring and creating a runtime.
// pub struct RuntimeBuilder {
//   execution_mode: ExecutionMode,
//   worker_threads: WorkerThreads,
//   enable_work_stealing: bool,
// }
//
// impl Default for RuntimeBuilder {
//   fn default() -> Self {
//     Self {
//       execution_mode: ExecutionMode::MultiThreaded,
//       enable_work_stealing: true,
//       worker_threads: WorkerThreads::default()
//     }
//   }
// }
//
// impl RuntimeBuilder {
//   /// Creates a new builder configured for single-threaded execution.
//   pub fn single_threaded() -> Self {
//     Self {
//       execution_mode: ExecutionMode::SingleThreaded,
//       enable_work_stealing: false,
//       worker_threads: WorkerThreads::Number(NonZero::new(1).unwrap()),
//     }
//   }
//
//   /// Creates a new builder configured for multi-threaded execution.
//   pub fn multi_threaded() -> Self {
//     Self {
//       execution_mode: ExecutionMode::MultiThreaded,
//       enable_work_stealing: true,
//       worker_threads: WorkerThreads::default(),
//     }
//   }
//
//   /// Sets the execution mode to single-threaded.
//   pub fn single_threaded_mode(mut self) -> Self {
//     self.execution_mode = ExecutionMode::SingleThreaded;
//     self.enable_work_stealing = false;
//     self.worker_threads = WorkerThreads::Number(NonZero::new(1).unwrap());
//     self
//   }
//
//   /// Sets the execution mode to multi-threaded.
//   pub fn multi_threaded_mode(mut self) -> Self {
//     self.execution_mode = ExecutionMode::MultiThreaded;
//     self.enable_work_stealing = true;
//     self
//   }
//
//   /// Sets the number of worker threads for multi-threaded execution.
//   ///
//   /// This method only has an effect when the runtime is in multi-threaded mode.
//   /// In single-threaded mode, this setting is ignored.
//   pub fn worker_threads(mut self, num: usize) -> Self {
//     if let Some(non_zero) = NonZero::new(num) {
//       self.worker_threads.set_threads(non_zero);
//     }
//     self
//   }
//
//   /// Disables work stealing for multi-threaded execution.
//   ///
//   /// This method only has an effect when the runtime is in multi-threaded mode.
//   /// In single-threaded mode, work stealing is always disabled.
//   pub fn disable_work_stealing(mut self) -> Self {
//     self.enable_work_stealing = false;
//     self
//   }
//
//   /// Enables work stealing for multi-threaded execution (default).
//   ///
//   /// This method only has an effect when the runtime is in multi-threaded mode.
//   pub fn enable_work_stealing(mut self) -> Self {
//     self.enable_work_stealing = true;
//     self
//   }
//
//   /// Returns the current execution mode.
//   pub fn execution_mode(&self) -> ExecutionMode {
//     self.execution_mode
//   }
//
//   /// Returns the number of worker threads that will be used.
//   pub fn worker_thread_count(&self) -> usize {
//     match self.execution_mode {
//       ExecutionMode::SingleThreaded => 1,
//       ExecutionMode::MultiThreaded => self.worker_threads.get_threads().get(),
//     }
//   }
//
//   pub(crate) fn get_num_workers(&self) -> NonZero<usize> {
//     match self.execution_mode {
//       ExecutionMode::SingleThreaded => NonZero::new(1).unwrap(),
//       ExecutionMode::MultiThreaded => self.worker_threads.get_threads(),
//     }
//   }
//
//   pub(crate) fn is_single_threaded(&self) -> bool {
//     self.execution_mode == ExecutionMode::SingleThreaded
//   }
//
//   pub(crate) fn is_work_stealing_enabled(&self) -> bool {
//     self.execution_mode == ExecutionMode::MultiThreaded && self.enable_work_stealing
//   }
//
//   /// Blocks the current thread until the future completes.
//   ///
//   /// This method creates a runtime with the current configuration and runs the
//   /// provided future to completion.
//   pub fn block_on<F, Res>(self, fut: F) -> Res
//   where
//     F: Future<Output = Res>,
//   {
//     Runtime::with_config(fut, self)
//   }
// }

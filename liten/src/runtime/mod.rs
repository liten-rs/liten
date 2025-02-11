mod main_executor;
pub(crate) mod scheduler;
mod waker;

use std::{
  future::Future,
  sync::{Arc, Mutex},
  task::{Context, RawWaker, RawWakerVTable, Waker},
};

use main_executor::GlobalExecutor;
use scheduler::{
  worker::{Shared, WorkersBuilder},
  Scheduler,
};

use crate::{
  context,
  io_loop::{self, TokenGenerator},
};

pub struct Runtime {
  scheduler: Scheduler,
  handle: Arc<scheduler::Handle>,
  driver: Arc<Mutex<scheduler::Driver>>,
}

impl Runtime {
  pub fn new() -> Self {
    let (io_driver, io_handle) = io_loop::Driver::new().unwrap();
    let shared = Shared::new(2);
    let handle = scheduler::Handle::new(io_handle, shared.clone());
    Runtime {
      scheduler: Scheduler,
      driver: Arc::new(Mutex::new(scheduler::Driver { io: io_driver })),
      handle: Arc::new(handle),
    }
  }

  pub fn block_on<F, Res>(self, fut: F) -> Res
  where
    F: Future<Output = Res>,
  {
    let workers = WorkersBuilder::from(self.handle.clone());

    context::runtime_enter(self.handle.clone(), |_| {
      workers.launch();

      let nice = self.driver.clone();
      let handle = self.handle.clone();
      let handle = std::thread::spawn(move || loop {
        let mut lock = nice.lock().unwrap();
        if lock.io.turn(handle.io()) {
          println!("whaat");
          break;
        }
      });
      let return_type = GlobalExecutor::block_on(fut);

      let waker = noop_waker();

      // Todo setup exist token register.
      self.handle.io().poll(
        TokenGenerator::new_wakeup().into(),
        &mut Context::from_waker(&waker),
      );
      handle.join().unwrap();

      return_type
    })
  }
}

unsafe fn noop(_data: *const ()) {}

const NOOP_WAKER_VTABLE: RawWakerVTable =
  RawWakerVTable::new(noop_clone, noop, noop, noop);
unsafe fn noop_clone(_data: *const ()) -> RawWaker {
  noop_raw_waker()
}
use core::ptr::null;
pub fn noop_waker() -> Waker {
  // FIXME: Since 1.46.0 we can use transmute in consts, allowing this function to be const.
  unsafe { Waker::from_raw(noop_raw_waker()) }
}

const fn noop_raw_waker() -> RawWaker {
  RawWaker::new(null(), &NOOP_WAKER_VTABLE)
}

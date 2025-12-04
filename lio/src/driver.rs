use crate::OperationProgress;
use crate::backends::{self, IoBackend};
use crate::op::Operation;
#[cfg(feature = "high")]
use crate::op_registration::TryExtractOutcome;

use parking_lot::Mutex;
#[cfg(feature = "high")]
use std::task::Waker;
use std::{
  collections::HashMap,
  ptr::NonNull,
  sync::{
    OnceLock,
    atomic::{AtomicPtr, AtomicU64, Ordering},
    mpsc,
  },
  thread,
};

use crate::op;
use crate::op_registration::OpRegistration;

pub struct OpStore {
  store: Mutex<HashMap<u64, OpRegistration>>,
}

impl OpStore {
  fn new() -> OpStore {
    Self { store: Mutex::new(HashMap::with_capacity(512)) }
  }
  pub fn next_id(&self) -> u64 {
    static NEXT: OnceLock<AtomicU64> = OnceLock::new();
    NEXT.get_or_init(|| AtomicU64::new(0)).fetch_add(1, Ordering::AcqRel)
  }
  pub fn remove(&self, id: u64) -> bool {
    let mut _lock = self.store.lock();
    _lock.remove(&id).is_some()
  }
  pub fn get_mut<F, R>(&self, id: u64, mut _f: F) -> Option<R>
  where
    F: FnOnce(&mut OpRegistration) -> R,
  {
    let mut _lock = self.store.lock();
    let reg = _lock.get_mut(&id)?;
    Some(_f(reg))
  }
  pub fn insert<O>(&self, id: u64, op: Box<O>)
  where
    O: Operation,
  {
    let mut _lock = self.store.lock();
    let result = _lock.insert(id, OpRegistration::new(op));
    assert!(result.is_none());
    drop(_lock);
  }
}

#[cfg(linux)]
use crate::op_registration::OpRegistrationStatus;

#[cfg(linux)]
pub type Default = backends::IoUring;
#[cfg(not(linux))]
pub type Default = backends::Polling;

pub(crate) struct Driver<Io = Default> {
  driver: Io,
  store: OpStore,
  // Shared shutdown state and background thread handle
  shutting_down: Mutex<Option<mpsc::Sender<()>>>,
  background_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

static DRIVER: AtomicPtr<Driver> = AtomicPtr::new(std::ptr::null_mut());

impl Driver {
  pub(crate) fn init() {
    // Check if already initialized
    let current = DRIVER.load(Ordering::Acquire);
    if !current.is_null() {
      panic!("Driver already initialized.");
    }

    let driver = Driver {
      driver: Default::new(),
      store: OpStore::new(),
      shutting_down: Mutex::new(None),
      background_handle: Mutex::new(None),
    };

    let driver_ptr = Box::into_raw(Box::new(driver));

    // Try to set the driver pointer atomically
    match DRIVER.compare_exchange(
      std::ptr::null_mut(),
      driver_ptr,
      Ordering::AcqRel,
      Ordering::Acquire,
    ) {
      Ok(_) => {}
      Err(_) => {
        // Another thread initialized first, clean up our allocation
        unsafe {
          let _ = Box::from_raw(driver_ptr);
        }
        panic!("Driver already initialized (race condition).");
      }
    }
  }

  pub(crate) fn get() -> &'static Driver {
    let ptr = DRIVER.load(Ordering::Acquire);
    if ptr.is_null() {
      panic!("Driver not initialized. Call Driver::init() first.");
    }
    // SAFETY: The pointer is valid from init() until deallocate()
    unsafe { &*ptr }
  }

  pub fn exit() {
    Driver::get().worker_shutdown();
    let ptr = NonNull::new(DRIVER.swap(std::ptr::null_mut(), Ordering::AcqRel))
      .expect("driver not initialized");
    Self::deallocate(ptr);
  }

  pub(crate) fn worker_shutdown(&'static self) {
    let mut handle_lock = self.background_handle.lock();

    if let Some(handle) = handle_lock.take() {
      // driver.poller.
    };
  }

  /// Deallocates the Driver, freeing all resources.
  /// This will panic if the driver is not initialized or if shutdown has not been called first.
  pub(crate) fn deallocate(ptr: NonNull<Driver>) {
    // SAFETY: This pointer was created via Box::into_raw in init()
    unsafe {
      let _ = Box::from_raw(ptr.as_ptr());
    }
  }

  pub(crate) fn detach(&self, id: u64) -> Option<()> {
    todo!();
  }
  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    let driver = Driver::get();
    driver.driver.submit(op, &driver.store)
  }

  pub(crate) fn tick(&self, can_wait: bool) {
    self.driver.tick(&self.store, can_wait)
  }

  pub fn spawn_ev(&'static self, sender: mpsc::Receiver<()>) {
    let handle = utils::create_worker(move || {
      loop {
        match sender.try_recv() {
          Ok(()) => break,
          Err(err) => match err {
            mpsc::TryRecvError::Empty => {}
            mpsc::TryRecvError::Disconnected => {
              break;
            }
          },
        }

        self.driver.tick(&self.store, true);
      }
    });
    let old = self.background_handle.lock().replace(handle);
    assert!(old.is_none());
  }

  // FIXME: On first run per key, run run_blocking and that will fix it.
  #[cfg(feature = "high")]
  pub fn check_done<T>(&self, key: u64) -> TryExtractOutcome<T::Result>
  where
    T: Operation,
  {
    match self.store.get_mut(key, |entry| entry.try_extract::<T>()).unwrap() {
      TryExtractOutcome::Done(res) => {
        self.store.remove(key);
        TryExtractOutcome::Done(res)
      }
      TryExtractOutcome::StillWaiting => TryExtractOutcome::StillWaiting,
    }
  }
  pub fn set_callback<T, F>(&self, id: u64, callback: F)
  where
    T: op::Operation,
    F: FnOnce(T::Result) + Send,
  {
    use crate::op_registration::OpCallback;
    self
      .store
      .get_mut(id, |entry| {
        entry.set_callback(OpCallback::new::<T, F>(callback))
      })
      .unwrap()
  }

  #[cfg(feature = "high")]
  pub(crate) fn set_waker(&self, id: u64, waker: Waker) {
    self.store.get_mut(id, |entry| entry.set_waker(waker)).unwrap()
  }
}

mod utils {
  use std::thread;

  pub fn create_worker<F, T>(handle: F) -> thread::JoinHandle<T>
  where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
  {
    thread::Builder::new()
      .name("lio".into())
      .spawn(handle)
      .expect("failed to launch the worker thread")
  }
}

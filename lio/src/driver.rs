use crate::OperationProgress;
use std::{
  collections::HashMap,
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  thread::JoinHandle,
};
#[cfg(not(linux))]
use std::{io, os::fd::RawFd, task::Waker};
#[cfg(linux)]
use std::{sync::atomic::AtomicBool, task::Waker};

#[cfg(linux)]
use io_uring::{IoUring, Probe};

#[cfg(not(linux))]
use crate::op;
use crate::op_registration::OpRegistration;
#[cfg(linux)]
use crate::op_registration::OpRegistrationStatus;

pub(crate) struct Driver(Arc<DriverInner>);

struct DriverInner {
  #[cfg(linux)]
  inner: IoUring,

  #[cfg(linux)]
  probe: Probe,

  #[cfg(linux)]
  has_done_work: AtomicBool,
  #[cfg(linux)]
  submission_guard: Mutex<()>,

  #[cfg(not_linux)]
  poller: polling::Poller,

  wakers: Mutex<HashMap<u64, OpRegistration>>,
  // Shared shutdown state and background thread handle
  shutting_down: AtomicBool,
  background_handle: Mutex<Option<JoinHandle<()>>>,
}
impl Driver {
  fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::AcqRel)
  }

  pub(crate) fn get() -> &'static Driver {
    static DRIVER: OnceLock<Driver> = OnceLock::new();

    DRIVER.get_or_init(|| {
      #[cfg(linux)]
      let (io_uring, probe) = {
        let io_uring = IoUring::new(256).unwrap();
        let mut probe = Probe::new();
        io_uring.submitter().register_probe(&mut probe).unwrap();
        (io_uring, probe)
      };

      let driver = Driver(Arc::new(DriverInner {
        #[cfg(linux)]
        inner: io_uring,

        #[cfg(linux)]
        probe,

        #[cfg(linux)]
        submission_guard: Mutex::new(()),
        #[cfg(linux)]
        has_done_work: AtomicBool::new(false),

        #[cfg(not_linux)]
        poller: polling::Poller::new().unwrap(),

        wakers: Mutex::new(HashMap::default()),
        shutting_down: AtomicBool::new(false),
        background_handle: Mutex::new(None),
      }));

      driver.background();

      driver
    })
  }

  pub(crate) fn shutdown() {
    static DONE_BEFORE: OnceLock<()> = OnceLock::new();
    if DONE_BEFORE.get().is_some() {
      return;
    }

    let driver = Driver::get();
    driver.0.shutting_down.store(true, Ordering::Release);

    #[cfg(not_linux)]
    {
      // Wake the poller so it can observe the shutdown flag
      let _ = driver.0.poller.notify();
    };
    #[cfg(linux)]
    {
      // Submit a NOP to wake submit_and_wait
      unsafe {
        let _g = driver.0.submission_guard.lock();
        let mut sub = driver.0.inner.submission_shared();
        let entry = io_uring::opcode::Nop::new().build().user_data(0);
        let _ = sub.push(&entry);
        sub.sync();
        drop(sub);
      }
      let _ = driver.0.inner.submit();
    };

    if let Some(handle) = driver.0.background_handle.lock().unwrap().take() {
      let _ = handle.join();
    }

    let _ = DONE_BEFORE.set(());
  }
}

#[cfg(linux)]
pub(crate) enum CheckRegistrationResult<V> {
  /// Waker has been registered and future should return Poll::Pending
  WakerSet,
  /// Value has been returned and future should poll anymore.
  Value(V),
}

#[cfg(linux)]
impl Driver {
  pub(crate) fn detach(&self, id: u64) -> Option<()> {
    let mut _lock = Driver::get().0.wakers.lock().unwrap();

    let thing = _lock.get_mut(&id)?;
    thing.status = OpRegistrationStatus::Cancelling;

    Some(())
  }

  pub fn background(&self) {
    // SAFETY: completion_shared is only accessed here so it's a singlethreaded access, hence
    // guaranteed only to have one completion queue.
    let driver = self.0.clone();
    let handle = utils::create_worker(move || {
      loop {
        use io_uring::cqueue::Entry;

        if driver.shutting_down.load(Ordering::Acquire) {
          break;
        }
        driver.inner.submit_and_wait(1).unwrap();

        let entries: Vec<Entry> =
            // SAFETY: The only thread that is concerned with completion queue.
            unsafe { driver.inner.completion_shared() }.collect();

        for entry in entries {
          use std::mem;

          let operation_id = entry.user_data();

          let mut wakers = driver.wakers.lock().unwrap();

          // If the operation id is not registered (e.g., wake-up NOP), skip.
          let Some(op_registration) = wakers.get_mut(&operation_id) else {
            continue;
          };

          let old_value = mem::replace(
            &mut op_registration.status,
            OpRegistrationStatus::Done { ret: entry.result() },
          );

          match old_value {
            OpRegistrationStatus::Waiting { ref registered_waker } => {
              if let Some(waker) = registered_waker.take() {
                waker.wake();
              };
            }
            OpRegistrationStatus::Cancelling => {
              let reg = wakers.remove(&operation_id).unwrap();

              // Dropping the operation.
              (reg.drop_fn)(reg.op);
            }
            OpRegistrationStatus::Done { .. } => {
              unreachable!("already processed entry");
            }
          };
        }
        unsafe { driver.inner.completion_shared() }.sync();
      }
    });

    *self.0.background_handle.lock().unwrap() = Some(handle);
  }
  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    let driver = Self::get();
    if T::entry_supported(&driver.0.probe) {
      let operation_id = driver.push::<T>(op);
      OperationProgress::<T>::new_uring(operation_id)
    } else {
      OperationProgress::<T>::new_blocking(op)
    }
  }
  fn push<T: op::Operation>(&self, op: T) -> u64 {
    let operation_id = Self::next_id();
    let entry = op.create_entry().user_data(operation_id);

    let mut _lock = self.0.wakers.lock().unwrap();

    // SAFETY: because of references rules, a "fake" lock has to be implemented here, but because
    // of it, this is safe.
    let _g = self.0.submission_guard.lock().unwrap();
    unsafe {
      let mut sub = self.0.inner.submission_shared();
      // FIXME
      sub.push(&entry).expect("unwrapping for now");
      sub.sync();
      drop(sub);
    }
    drop(_g);

    _lock.insert(operation_id, OpRegistration::new(op));
    self.0.inner.submit().unwrap();
    self.0.has_done_work.store(true, Ordering::SeqCst);

    operation_id
  }

  pub(crate) fn check_registration<T: op::Operation>(
    &self,
    id: u64,
    waker: Waker,
  ) -> Option<CheckRegistrationResult<T::Result>> {
    let mut _lock = self.0.wakers.lock().unwrap();
    let op_registration = _lock.get_mut(&id)?;

    Some(match op_registration.status {
      OpRegistrationStatus::Done { ret } => {
        let op_registration = _lock.remove(&id).expect("what");

        // SAFETY: The pointer was created with Box::into_raw in queue_submit with a concrete type T
        // We can safely cast it back to the concrete type T
        let mut value = unsafe { Box::from_raw(op_registration.op as *mut T) };

        let raw_ret = if ret < 0 {
          use std::io;

          Err(io::Error::from_raw_os_error(-ret))
        } else {
          Ok(ret)
        };

        CheckRegistrationResult::Value(value.result(raw_ret))
      }
      OpRegistrationStatus::Waiting { ref mut registered_waker } => {
        registered_waker.replace(Some(waker));
        CheckRegistrationResult::WakerSet
      }
      OpRegistrationStatus::Cancelling => {
        unreachable!("wtf to do here?");
      }
    })
  }
}

#[cfg(not(linux))]
impl Driver {
  // pub(crate) fn detach(&self, id: u64) -> Option<()> {
  //   let mut _lock = Driver::get().0.wakers.lock().unwrap();
  //
  //   let thing = _lock.remove(&id)?;
  //   // If exists:
  //
  //   // SAFETY: Just turning a RawFd into something polling crate can understand.
  //   let fd = unsafe {
  //     use std::os::fd::BorrowedFd;
  //     BorrowedFd::borrow_raw(thing.fd())
  //   };
  //   self.0.poller.delete(fd).unwrap();
  //
  //   Some(())
  // }
  // pub(crate) fn insert_poll(&self, fd: RawFd, interest: PollInterest) -> u64 {
  //   let mut _lock = self.0.wakers.lock().unwrap();
  //   let id = Self::next_id();
  //
  //   let op = OpRegistration::new(fd, interest);
  //   let _ = _lock.insert(id, op);
  //
  //   // SAFETY: Just turning a RawFd into something polling crate can understand.
  //   unsafe {
  //     use std::os::fd::BorrowedFd;
  //
  //     let fd = BorrowedFd::borrow_raw(fd);
  //     self.0.poller.add(&fd, interest.as_event(id)).unwrap();
  //   };
  //
  //   id
  // }

  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    if T::EVENT_TYPE.is_some() {
      OperationProgress::<T>::new(Driver::get().reserve_driver_entry(), op)
    } else {
      OperationProgress::<T>::new_blocking(op)
    }
  }

  /// Returns None operation should be run blocking.
  fn reserve_driver_entry(&self) -> u64 {
    let operation_id = Self::next_id();

    let mut _lock = self.0.wakers.lock().unwrap();

    _lock.insert(operation_id, OpRegistration::new_without_waker());

    operation_id
  }
  // // pub(crate) fn submit_block<O: op::Operation>(op: O) -> OperationProgress<O> {
  // //   use crate::OperationProgress;
  // //
  // //   OperationProgress::new_blocking(op)
  // // }
  // pub(crate) fn submit_poll<O: op::Operation>(
  //   fd: RawFd,
  //   interest: PollInterest,
  //   op: O,
  // ) -> OperationProgress<O> {
  //   let id = Driver::get().insert_poll(fd, interest);
  //   OperationProgress::new_poll(id, op)
  // }

  pub(crate) fn register_repoll(
    &self,
    key: u64,
    event: polling::Event,
    waker: Waker,
    fd: RawFd,
  ) -> Option<io::Result<()>> {
    use std::os::fd::BorrowedFd;

    let mut _lock = self.0.wakers.lock().unwrap();
    let _ = _lock.remove(&key)?;
    drop(_lock);

    if let Err(err) =
      unsafe { self.0.poller.add(&BorrowedFd::borrow_raw(fd), event) }
    {
      return Some(Err(err));
    };

    let mut _lock = self.0.wakers.lock().unwrap();
    _lock.insert(key, OpRegistration::new_with_waker(waker));
    drop(_lock);

    Some(Ok(()))
  }

  pub fn background(&self) {
    let driver = self.0.clone();
    let handle = utils::create_worker(move || {
      let mut events = polling::Events::new();
      loop {
        if driver.shutting_down.load(Ordering::Acquire) {
          break;
        }
        events.clear();
        driver.poller.wait(&mut events, None).unwrap();

        if driver.shutting_down.load(Ordering::Acquire) {
          break;
        }

        let mut _lock = driver.wakers.lock().unwrap();
        for event in events.iter() {
          if let Some(reg) = _lock.get_mut(&(event.key as _)) {
            reg.wake();
          }
        }
      }
    });

    *self.0.background_handle.lock().unwrap() = Some(handle);
  }
}

mod utils {
  use std::thread::{self, JoinHandle};

  pub fn create_worker<F, T>(handle: F) -> JoinHandle<T>
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

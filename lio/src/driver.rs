use crate::OperationProgress;
use std::collections::HashMap;

use crate::loom::{
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
  },
  thread::JoinHandle,
};

#[cfg(not(linux))]
use std::{io, os::fd::RawFd, task::Waker};
#[cfg(linux)]
use std::{sync::atomic::AtomicBool, task::Waker};

#[cfg(not(linux))]
use polling::PollMode;

#[cfg(linux)]
use io_uring::{IoUring, Probe};
use tokio::sync::oneshot;

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

  #[cfg(not(linux))]
  poller: polling::Poller,

  wakers: Mutex<HashMap<u64, OpRegistration>>,
  // Shared shutdown state and background thread handle
  shutting_down: Mutex<Option<oneshot::Sender<()>>>,
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

        #[cfg(not(linux))]
        poller: polling::Poller::new().unwrap(),

        wakers: Mutex::new(HashMap::default()),
        shutting_down: Mutex::new(None),
        background_handle: Mutex::new(None),
      }));

      driver.background();

      driver
    })
  }

  pub(crate) async fn shutdown() {
    #[cfg(feature = "tracing")]
    tracing::info!("before");

    static DONE_BEFORE: OnceLock<()> = OnceLock::new();
    if DONE_BEFORE.get().is_some() {
      #[cfg(feature = "tracing")]
      tracing::info!("already shut down, returning");
      return;
    }

    let driver = Driver::get();
    #[cfg(feature = "tracing")]
    tracing::info!("got driver");
    let (sender, receiver) = oneshot::channel();

    #[cfg(feature = "tracing")]
    tracing::info!("before lock");
    let mut _lock = driver.0.shutting_down.lock();
    #[cfg(feature = "tracing")]
    tracing::info!("after lock");
    assert!(_lock.replace(sender).is_none());
    drop(_lock);

    #[cfg(not(linux))]
    {
      // Wake the poller so it can observe the shutdown flag
      let _ = driver.0.poller.notify().unwrap();
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

    #[cfg(feature = "tracing")]
    tracing::info!("before receiver");
    let _ = receiver.await.unwrap();

    let mut _lock = driver.0.background_handle.lock();

    let handle = _lock.take().unwrap();
    #[cfg(feature = "tracing")]
    tracing::info!("before joining");
    let _ = handle.join();

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
    let mut _lock = Driver::get().0.wakers.lock();

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

          let mut wakers = driver.wakers.lock();

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

    *self.0.background_handle.lock() = Some(handle);
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

    let mut _lock = self.0.wakers.lock();

    // SAFETY: because of references rules, a "fake" lock has to be implemented here, but because
    // of it, this is safe.
    let _g = self.0.submission_guard.lock();
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
    let mut _lock = self.0.wakers.lock();
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
  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    #[cfg(feature = "tracing")]
    tracing::debug!("submitting op");
    if T::EVENT_TYPE.is_some() {
      let fd = op.fd().expect("operation has event_type but no fd");
      OperationProgress::<T>::new(Driver::get().reserve_driver_entry(fd), op)
    } else {
      OperationProgress::<T>::new_blocking(op)
    }
  }

  /// Returns None operation should be run blocking.
  fn reserve_driver_entry(&self, fd: RawFd) -> u64 {
    let operation_id = Self::next_id();

    let mut _lock = self.0.wakers.lock();

    assert!(
      !_lock.contains_key(&operation_id),
      "operation_id {} collision - already exists!",
      operation_id
    );

    _lock.insert(operation_id, OpRegistration::new(fd));

    operation_id
  }

  pub(crate) fn register_repoll(
    &self,
    key: u64,
    event: polling::Event,
    waker: Waker,
    fd: RawFd,
  ) -> Option<io::Result<()>> {
    #[cfg(feature = "tracing")]
    tracing::debug!("register repoll");
    let mut _lock = self.0.wakers.lock();
    let registration = _lock
      .get_mut(&key)
      .expect("tried to register poll on operation which doesn't exist");

    assert!(fd == registration.fd, "provided fd and registration.fd not same");

    registration.set_waker(waker);

    // Try modify first if was registered, otherwise add
    #[cfg(feature = "tracing")]
    tracing::debug!("modifying or adding poller");

    use std::os::fd::BorrowedFd;
    let result = unsafe {
      self.0.poller.modify_with_mode(
        &BorrowedFd::borrow_raw(fd),
        event,
        PollMode::Oneshot,
      )
    };
    if let Err(e) = result {
      if e.kind() == io::ErrorKind::NotFound {
        // Fd not registered yet, add it
        unsafe {
          self.0.poller.add_with_mode(
            &BorrowedFd::borrow_raw(fd),
            event,
            PollMode::Oneshot,
          )
        }
        .expect("failed to add fd to poller");
      } else {
        // Some other error with modify
        return Some(Err(e));
      }
    }

    // Wake the background thread so it can monitor this new event
    let _ = self.0.poller.notify();

    Some(Ok(()))
  }

  pub(crate) fn detach(&self, key: u64) {
    let mut _lock = self.0.wakers.lock();
    if let Some(reg) = _lock.remove(&key) {
      if reg.has_waker() {
        let _ = unsafe {
          use std::os::fd::BorrowedFd;

          // Remove from poller to prevent stale events
          self.0.poller.delete(&BorrowedFd::borrow_raw(reg.fd)).unwrap();
        };
      }
    }
    // If registration doesn't exist, it was likely already processed and removed by background thread
  }

  pub fn background(&self) {
    let driver = self.0.clone();
    let handle = utils::create_worker(move || {
      #[cfg(feature = "tracing")]
      tracing::info!("launching bg task");
      let mut events = polling::Events::new();
      loop {
        #[cfg(feature = "tracing")]
        tracing::info!("locking");
        let mut _lock = driver.shutting_down.lock();

        if let Some(sender) = _lock.take() {
          #[cfg(feature = "tracing")]
          tracing::info!("breaking bg");
          sender.send(()).unwrap();
          break;
        };
        drop(_lock);

        events.clear();

        #[cfg(feature = "tracing")]
        tracing::debug!("waiting");

        // Under shuttle, use a small timeout to avoid busy-spinning while still
        // allowing shuttle to explore different schedules
        let wait_result = driver.poller.wait(&mut events, None);

        #[cfg(feature = "tracing")]
        tracing::debug!("got events");

        // Ignore timeout errors, just check shutdown flag
        if wait_result.is_err() {
          continue;
        }

        // Having zero events is normal (timeout or no I/O ready), just loop again
        for event in events.iter() {
          let mut _lock = driver.wakers.lock();
          let mut entry = match _lock.remove(&(event.key as u64)) {
            Some(entry) => {
              drop(_lock);
              entry
            }
            None => {
              drop(_lock);
              // Notification events from poller.notify() don't have waker entries, skip them
              #[cfg(feature = "tracing")]
              tracing::debug!(key = ?event.key, "skipping event with no waker (likely notification)");
              continue;
            }
          };

          #[cfg(feature = "tracing")]
          tracing::debug!(key = ?event.key, "woke progress");
          entry.wake();

          let mut _lock = driver.wakers.lock();
          unsafe {
            use std::os::fd::BorrowedFd;
            // // Delete the fd from the poller after waking, so it can be re-added on next poll
            let _ =
              driver.poller.delete(&BorrowedFd::borrow_raw(entry.fd)).unwrap();
          }
          _lock.insert(event.key as u64, entry);
        }
      }
    });

    *self.0.background_handle.lock() = Some(handle);
  }
}

mod utils {
  use crate::loom::thread;
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

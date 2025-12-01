use crate::OperationProgress;
#[cfg(not(linux))]
use crate::op::EventType;
use crate::op::Operation;
#[cfg(feature = "high")]
use crate::op_registration::TryExtractOutcome;

use parking_lot::Mutex;
#[cfg(not(linux))]
use std::os::fd::RawFd;
#[cfg(linux)]
use std::sync::atomic::AtomicBool;
#[cfg(feature = "high")]
use std::task::Waker;
use std::{
  collections::HashMap,
  io,
  sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
    mpsc,
  },
  thread,
};

#[cfg(linux)]
use io_uring::{IoUring, Probe};

use crate::op;
use crate::op_registration::{ExtractedOpNotification, OpRegistration};

#[cfg(linux)]
use crate::op_registration::OpRegistrationStatus;

pub(crate) struct Driver {
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
  shutting_down: Mutex<Option<mpsc::Sender<()>>>,
  background_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Driver {
  fn next_id() -> u64 {
    static NEXT: OnceLock<AtomicU64> = OnceLock::new();
    NEXT.get_or_init(|| AtomicU64::new(0)).fetch_add(1, Ordering::AcqRel)
  }

  pub(crate) fn get() -> &'static Driver {
    static DRIVER: OnceLock<Driver> = OnceLock::new();

    if let Some(driver) = DRIVER.get() {
      return driver;
    }

    #[cfg(linux)]
    let (io_uring, probe) = {
      let io_uring = IoUring::new(256).unwrap();
      let mut probe = Probe::new();
      io_uring.submitter().register_probe(&mut probe).unwrap();
      (io_uring, probe)
    };

    let driver = Driver {
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

      wakers: Mutex::new(HashMap::with_capacity(256)),
      shutting_down: Mutex::new(None),
      background_handle: Mutex::new(None),
    };

    if DRIVER.set(driver).is_err() {
      DRIVER.get().unwrap()
    } else {
      let driver = DRIVER.get().unwrap();
      let (sender, receiver) = mpsc::channel();

      #[cfg(feature = "tracing")]
      tracing::debug!("Driver::get: starting background thread");

      let handle = driver.background(receiver);
      *driver.background_handle.lock() = Some(handle);

      // Verify background thread handle was set
      {
        let handle_lock = driver.background_handle.lock();
        assert!(
          handle_lock.is_some(),
          "Driver::get: background thread handle was not set after background() call"
        );
      }

      #[cfg(feature = "tracing")]
      tracing::debug!(
        "Driver::get: background thread started, setting shutdown sender"
      );
      *driver.shutting_down.lock() = Some(sender);
      driver
    }
  }

  pub(crate) fn shutdown() {
    let driver = Driver::get();

    let mut _lock = driver.shutting_down.lock();
    let sender = _lock.take().expect("cannot find sender");
    drop(_lock);

    #[cfg(not(linux))]
    {
      // Wake the poller so it can observe the shutdown flag
      let _ = driver.poller.notify().unwrap();
    };
    #[cfg(linux)]
    {
      // Submit a NOP to wake submit_and_wait
      unsafe {
        let _g = driver.submission_guard.lock();
        let mut sub = driver.inner.submission_shared();
        let entry = io_uring::opcode::Nop::new().build().user_data(0);
        let _ = sub.push(&entry);
        sub.sync();
        drop(sub);
      }
      let _ = driver.inner.submit();
    };

    let _ = sender.send(()).unwrap();

    let mut _lock = driver.background_handle.lock();
    let handle = _lock.take().unwrap();
    drop(_lock);
    let _ = handle.join();
  }

  // FIXME: On first run per key, run run_blocking and that will fix it.
  #[cfg(feature = "high")]
  pub fn check_done<T>(&self, key: u64) -> TryExtractOutcome<T::Result>
  where
    T: Operation,
  {
    let mut _lock = self.wakers.lock();

    // Optimise for done being true.
    let entry = _lock.get_mut(&key).expect("key invalid");

    match entry.try_extract::<T>() {
      TryExtractOutcome::Done(res) => {
        let _ = _lock.remove(&key).expect("wtf?");
        drop(_lock);
        TryExtractOutcome::Done(res)
      }
      TryExtractOutcome::StillWaiting => TryExtractOutcome::StillWaiting,
      TryExtractOutcome::HasCancelled => TryExtractOutcome::HasCancelled,
    }
  }
  pub(crate) fn set_callback<T, F>(&self, id: u64, callback: F)
  where
    T: op::Operation,
    F: FnOnce(T::Result) + Send,
  {
    use crate::op_registration::OpCallback;

    let mut _lock = self.wakers.lock();
    let entry = _lock.get_mut(&id).unwrap();

    entry.set_callback(OpCallback::new::<T, F>(callback));
  }

  #[cfg(feature = "high")]
  pub(crate) fn set_waker(&self, id: u64, waker: Waker) {
    let mut _lock = self.wakers.lock();
    let entry = _lock.get_mut(&id).unwrap();

    entry.set_waker(waker);
  }
}

#[cfg(linux)]
impl Driver {
  pub(crate) fn detach(&self, id: u64) -> Option<()> {
    Some(())
  }

  pub fn background(
    &'static self,
    receiver: mpsc::Receiver<()>,
  ) -> thread::JoinHandle<()> {
    // SAFETY: completion_shared is only accessed here so it's a singlethreaded access, hence
    // guaranteed only to have one completion queue.
    utils::create_worker(move || {
      loop {
        use io_uring::cqueue::Entry;

        match receiver.try_recv() {
          Ok(()) => {
            #[cfg(feature = "tracing")]
            tracing::info!("background thread: shutdown signal received");
            break;
          }
          Err(err) => match err {
            mpsc::TryRecvError::Empty => {
              #[cfg(feature = "tracing")]
              tracing::info!("background thread, haven't seen");
            }
            mpsc::TryRecvError::Disconnected => {
              #[cfg(feature = "tracing")]
              tracing::info!("background thread: sender closed");
              break;
            }
          },
        };

        self.inner.submit_and_wait(1).unwrap();

        let entries: Vec<Entry> =
            // SAFETY: The only thread that is concerned with completion queue.
            unsafe { self.inner.completion_shared() }.collect();

        for io_entry in entries {
          use std::mem;

          let operation_id = io_entry.user_data();

          let mut _lock = self.wakers.lock();

          // If the operation id is not registered (e.g., wake-up NOP), skip.
          let Some(entry) = _lock.get_mut(&operation_id) else {
            continue;
          };

          // if should keep.
          match entry.set_done(utils::from_i32_to_io_result(io_entry.result()))
          {
            None => {}
            Some(value) => match value {
              #[cfg(feature = "high")]
              ExtractedOpNotification::Waker(waker) => {
                drop(_lock);
                waker.wake()
              }
              ExtractedOpNotification::Callback(callback) => {
                callback.call(entry);
                _lock.remove(&operation_id);
                drop(_lock);
              }
            },
          }
        }
        unsafe { self.inner.completion_shared() }.sync();
      }
    })
  }
  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    let driver = Self::get();
    if T::entry_supported(&driver.probe) {
      let operation_id = Self::next_id();

      let mut op = Box::new(op);
      let entry = op.create_entry().user_data(operation_id);

      // Insert the operation into wakers first
      {
        let mut _lock = driver.wakers.lock();
        _lock.insert(operation_id, OpRegistration::new(op));
      }

      // Then submit to io_uring
      // SAFETY: because of references rules, a "fake" lock has to be implemented here, but because
      // of it, this is safe.
      let _g = driver.submission_guard.lock();
      unsafe {
        let mut sub = driver.inner.submission_shared();
        // FIXME
        sub.push(&entry).expect("unwrapping for now");
        sub.sync();
        drop(sub);
      }
      drop(_g);

      driver.inner.submit().unwrap();
      driver.has_done_work.store(true, Ordering::SeqCst);
      OperationProgress::<T>::new_uring(operation_id)
    } else {
      // TODO:
      // OperationProgress::<T>::new_blocking(op)

      unimplemented!("")
    }
  }
}

#[cfg(not(linux))]
impl Driver {
  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    #[cfg(not(linux))]
    fn new_polling<T>(op: T) -> OperationProgress<T>
    where
      T: Operation,
    {
      let driver = Driver::get();
      let fd = op.fd().expect("not provided fd");
      let operation_id = driver.reserve_driver_entry(Box::new(op), fd);
      #[cfg(feature = "tracing")]
      tracing::debug!(
        operation_id = operation_id,
        fd,
        operation = std::any::type_name::<T>(),
        "submit: created polling operation"
      );
      let _ = Driver::add_interest(
        &driver.poller,
        fd,
        operation_id,
        T::EVENT_TYPE.expect("op is event but no event_type??"),
      )
      .expect("fd sure exists");
      OperationProgress::<T>::new_polling(operation_id)
    }

    if T::EVENT_TYPE.is_none() {
      #[cfg(feature = "tracing")]
      tracing::debug!(
        operation = std::any::type_name::<T>(),
        "submit: created blocking operation"
      );
      return OperationProgress::<T>::new_blocking(op);
    };

    if !T::IS_CONNECT {
      return new_polling(op);
    };

    let result = op.run_blocking();

    if result
      .as_ref()
      .is_err_and(|err| err.raw_os_error() == Some(libc::EINPROGRESS))
    {
      new_polling(op)
    } else {
      OperationProgress::<T>::new_from_result(op, result)
    }
  }

  /// Reserves a driver entry and stores the operation for later execution
  fn reserve_driver_entry<T>(&self, op: Box<T>, fd: RawFd) -> u64
  where
    T: op::Operation,
  {
    assert!(fd > 0, "reserve_driver_entry: invalid fd {}", fd);

    let operation_id = Self::next_id();

    let mut _lock = self.wakers.lock();

    assert!(
      !_lock.contains_key(&operation_id),
      "reserve_driver_entry: operation_id {} collision - already exists! (ID generation bug)",
      operation_id
    );

    _lock.insert(operation_id, OpRegistration::new(op, fd));

    // Verify insertion
    assert!(
      _lock.contains_key(&operation_id),
      "reserve_driver_entry: failed to insert operation_id {}",
      operation_id
    );

    #[cfg(feature = "tracing")]
    tracing::debug!(
      operation_id = operation_id,
      fd = fd,
      "reserved driver entry with operation"
    );

    operation_id
  }

  pub(crate) fn detach(&self, key: u64) {}

  pub(crate) fn add_interest(
    poller: &polling::Poller,
    fd: RawFd,
    key: u64,
    _event: EventType,
  ) -> io::Result<()> {
    let event = match _event {
      EventType::Read => polling::Event::readable(key as usize),
      EventType::Write => polling::Event::writable(key as usize),
    };

    #[cfg(feature = "tracing")]
    tracing::info!(fd, key, event = ?&_event, "add interest");

    unsafe {
      use std::os::fd::BorrowedFd;

      poller.add(&BorrowedFd::borrow_raw(fd), event)
    }
  }
  pub(crate) fn modify_interest(
    poller: &polling::Poller,
    fd: RawFd,
    event: polling::Event,
  ) -> io::Result<()> {
    #[cfg(feature = "tracing")]
    tracing::info!(fd, event = ?&event, "modify interest");
    unsafe {
      use std::os::fd::BorrowedFd;

      poller.modify(&BorrowedFd::borrow_raw(fd), event)
    }
  }

  pub(crate) fn delete_interest(
    poller: &polling::Poller,
    fd: RawFd,
  ) -> io::Result<()> {
    #[cfg(feature = "tracing")]
    tracing::info!(fd, "modify interest");
    unsafe {
      use std::os::fd::BorrowedFd;

      poller.delete(&BorrowedFd::borrow_raw(fd))
    }
  }
  pub(crate) fn interest_wait(
    poller: &polling::Poller,
  ) -> io::Result<polling::Events> {
    let mut events = polling::Events::new();
    let _ = poller.wait(&mut events, None)?;
    Ok(events)
  }

  pub fn background(
    &'static self,
    sender: mpsc::Receiver<()>,
  ) -> thread::JoinHandle<()> {
    utils::create_worker(move || {
      #[cfg(feature = "tracing")]
      tracing::info!("background thread: started");
      loop {
        match sender.try_recv() {
          Ok(()) => {
            #[cfg(feature = "tracing")]
            tracing::info!("background thread: shutdown signal received");
            break;
          }
          Err(err) => match err {
            mpsc::TryRecvError::Empty => {
              #[cfg(feature = "tracing")]
              tracing::info!("background thread, haven't seen");
            }
            mpsc::TryRecvError::Disconnected => {
              #[cfg(feature = "tracing")]
              tracing::info!("background thread: sender closed");
              break;
            }
          },
        }

        #[cfg(feature = "tracing")]
        tracing::trace!("background thread: waiting on poller");

        let events =
          Self::interest_wait(&self.poller).expect("background thread failed");

        #[cfg(feature = "tracing")]
        if events.len() > 0 {
          tracing::debug!(
            event_count = events.len(),
            "background thread: received events"
          );
        }

        for event in events.iter() {
          let operation_id = event.key as u64;
          #[cfg(feature = "tracing")]
          tracing::debug!(
            operation_id = operation_id,
            "background thread: processing event"
          );

          let mut _lock = self.wakers.lock();
          let entry = _lock.get_mut(&operation_id).unwrap();

          let result = entry.run_blocking();

          match result {
            // Special-case.
            Err(err)
              if err.kind() == io::ErrorKind::WouldBlock
                || err.raw_os_error() == Some(libc::EINPROGRESS) =>
            {
              let _ = Self::modify_interest(&self.poller, entry.fd(), event)
                .expect("fd sure exists");

              continue;
            }
            _ => {
              let entry = _lock
                .get_mut(&operation_id)
                .expect("Cannot find matching operation");

              #[cfg(not(linux))]
              Self::delete_interest(&self.poller, entry.fd()).unwrap();

              // if should keep.
              match entry.set_done(result) {
                None => {}
                Some(value) => match value {
                  #[cfg(feature = "high")]
                  ExtractedOpNotification::Waker(waker) => {
                    drop(_lock);
                    waker.wake()
                  }
                  ExtractedOpNotification::Callback(callback) => {
                    callback.call(entry);
                    _lock.remove(&operation_id);
                    drop(_lock);
                  }
                },
              }
            }
          };
          #[cfg(feature = "tracing")]
          tracing::debug!(
            operation_id = operation_id,
            "background thread: waker woken"
          );
        }
      }
      #[cfg(feature = "tracing")]
      tracing::info!("background thread: exiting");
    })
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

  #[cfg(linux)]
  pub fn from_i32_to_io_result(res: i32) -> std::io::Result<i32> {
    if res < 0 { Err(std::io::Error::from_raw_os_error(res)) } else { Ok(res) }
  }
}

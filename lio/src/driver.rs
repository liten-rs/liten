use crate::OperationProgress;

use parking_lot::Mutex;
use std::{
  collections::HashMap,
  sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
    mpsc,
  },
  thread,
};
#[cfg(not(linux))]
use std::{io, os::fd::RawFd, task::Waker};
#[cfg(linux)]
use std::{sync::atomic::AtomicBool, task::Waker};

#[cfg(not(linux))]
use polling::PollMode;

#[cfg(linux)]
use io_uring::{IoUring, Probe};

use crate::op;
use crate::op_registration::OpRegistration;

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
    use std::mem;

    let mut _lock = Driver::get().wakers.lock();

    let thing = _lock.get_mut(&id)?;

    let old = mem::replace(&mut thing.status, OpRegistrationStatus::Cancelling);

    match old {
      OpRegistrationStatus::Waiting { ref registered_waker } => {
        println!("det is waiting");
        if let Some(waker) = registered_waker.take() {
          waker.wake();
        };
      }
      OpRegistrationStatus::Cancelling => {
        unreachable!("already was cancelling.");
      }
      OpRegistrationStatus::Done { .. } => {
        println!("det is done");
        // We don't care here because we shouldn't cancel anything, we just don't care about the
        // result anymore.
      }
    };

    Some(())
  }

  pub fn background(
    &'static self,
    mut receiver: mpsc::Receiver<()>,
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

        for entry in entries {
          use std::mem;

          let operation_id = entry.user_data();

          let mut wakers = self.wakers.lock();

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
              println!("is waiting");
              if let Some(waker) = registered_waker.take() {
                waker.wake();
              };
            }
            OpRegistrationStatus::Cancelling => {
              println!("cancelling");
              let reg = wakers.remove(&operation_id).unwrap();

              // Dropping the operation.
              (reg.drop_fn)(reg.op);
            }
            OpRegistrationStatus::Done { .. } => {
              println!("done");
              unreachable!("already processed entry");
            }
          };
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
      let operation_id = driver.push::<T>(op);
      OperationProgress::<T>::new_uring(operation_id)
    } else {
      OperationProgress::<T>::new_blocking(op)
    }
  }
  fn push<T: op::Operation>(&self, op: T) -> u64 {
    let operation_id = Self::next_id();
    let entry = op.create_entry().user_data(operation_id);

    // Insert the operation into wakers first
    {
      let mut _lock = self.wakers.lock();
      _lock.insert(operation_id, OpRegistration::new(op));
    }

    // Then submit to io_uring
    // SAFETY: because of references rules, a "fake" lock has to be implemented here, but because
    // of it, this is safe.
    let _g = self.submission_guard.lock();
    unsafe {
      let mut sub = self.inner.submission_shared();
      // FIXME
      sub.push(&entry).expect("unwrapping for now");
      sub.sync();
      drop(sub);
    }
    drop(_g);

    self.inner.submit().unwrap();
    self.has_done_work.store(true, Ordering::SeqCst);

    operation_id
  }

  pub(crate) fn check_registration<T>(
    &self,
    id: u64,
    waker: Waker,
  ) -> Option<CheckRegistrationResult<T::Result>>
  where
    T: op::Operation,
  {
    let mut _lock = self.wakers.lock();
    let op_registration = _lock.get_mut(&id)?;

    let res = match op_registration.status {
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
    };

    Some(res)
  }
}

#[cfg(not(linux))]
impl Driver {
  pub(crate) fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    if T::EVENT_TYPE.is_some() {
      let fd = op.fd().expect("operation has event_type but no fd");
      let operation_id = Driver::get().reserve_driver_entry(fd);
      #[cfg(feature = "tracing")]
      tracing::debug!(
        operation_id = operation_id,
        fd = fd,
        operation = std::any::type_name::<T>(),
        "submit: created polling operation"
      );
      OperationProgress::<T>::new(operation_id, op)
    } else {
      #[cfg(feature = "tracing")]
      tracing::debug!(
        operation = std::any::type_name::<T>(),
        "submit: created blocking operation"
      );
      OperationProgress::<T>::new_blocking(op)
    }
  }

  /// Returns None operation should be run blocking.
  fn reserve_driver_entry(&self, fd: RawFd) -> u64 {
    assert!(fd > 0, "reserve_driver_entry: invalid fd {}", fd);

    let operation_id = Self::next_id();

    let mut _lock = self.wakers.lock();

    assert!(
      !_lock.contains_key(&operation_id),
      "reserve_driver_entry: operation_id {} collision - already exists! (ID generation bug)",
      operation_id
    );

    _lock.insert(operation_id, OpRegistration::new(fd));

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
      "reserved driver entry"
    );

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
    tracing::debug!(operation_id = key, fd = fd, "register_repoll: starting");

    // Validate inputs
    assert!(fd > 0, "register_repoll: invalid fd {}", fd);

    // Set the waker first, then release the lock before doing poller operations
    {
      let mut _lock = self.wakers.lock();
      #[cfg(feature = "tracing")]
      tracing::trace!(
        operation_id = key,
        "register_repoll: acquired wakers lock"
      );

      let registration = _lock
        .get_mut(&key)
        .unwrap_or_else(|| {
          panic!(
            "register_repoll: operation_id {} does not exist in wakers map (race condition or double detach?)",
            key
          );
        });

      assert!(
        fd == registration.fd,
        "register_repoll: fd mismatch - provided {}, registration has {}",
        fd,
        registration.fd
      );

      // Check if a waker already exists. If it does, we update it with the new one.
      // This can happen legitimately if:
      // 1. The future is polled spuriously by the executor before the background thread processes the event
      // 2. The context/waker has changed between polls
      // The background thread will take the waker when it processes the event, so we need to ensure
      // the most recent waker is always set.
      let _had_waker = registration.has_waker();

      #[cfg(feature = "tracing")]
      tracing::trace!(
        operation_id = key,
        had_waker = _had_waker,
        "register_repoll: setting waker (replacing if already set)"
      );

      registration.set_waker(waker);

      // Verify waker was set
      assert!(
        registration.has_waker(),
        "register_repoll: waker was not set after set_waker() call"
      );
    };

    #[cfg(feature = "tracing")]
    tracing::trace!(
      operation_id = key,
      "register_repoll: released wakers lock, modifying poller"
    );

    // Verify background thread is running before modifying poller
    {
      let handle_lock = self.background_handle.lock();
      assert!(
        handle_lock.is_some(),
        "register_repoll: background thread not running (handle is None) - cannot register poll"
      );
    }

    // Now do poller operations without holding the wakers lock.
    // CRITICAL: We must hold the wakers lock while modifying poller to prevent
    // the background thread from processing events before the waker is set.
    // Actually, we already set the waker above, so the background thread can
    // safely process events. But we need to ensure the poller modification
    // happens atomically with respect to event processing.
    //
    // The issue is: if we modify poller after setting waker, there's a window
    // where the background thread could process an event but the poller isn't
    // updated yet. However, with Oneshot mode, once an event fires, the fd
    // is removed, so we need to re-add it. This is fine.
    use std::os::fd::BorrowedFd;

    let result = unsafe {
      self.poller.modify_with_mode(
        &BorrowedFd::borrow_raw(fd),
        event,
        PollMode::Oneshot,
      )
    };
    if let Err(e) = result {
      if e.kind() == io::ErrorKind::NotFound {
        #[cfg(feature = "tracing")]
        tracing::debug!(
          operation_id = key,
          fd = fd,
          "register_repoll: fd not in poller, adding"
        );
        // Fd not registered yet (or was removed by Oneshot mode after event fired), add it
        unsafe {
          self.poller.add_with_mode(
            &BorrowedFd::borrow_raw(fd),
            event,
            PollMode::Oneshot,
          )
        }
        .unwrap_or_else(|e| {
          panic!(
            "register_repoll: failed to add fd {} to poller for operation_id {}: {:?}",
            fd, key, e
          );
        });
        #[cfg(feature = "tracing")]
        tracing::debug!(
          operation_id = key,
          fd = fd,
          "register_repoll: added to poller"
        );
      } else {
        #[cfg(feature = "tracing")]
        tracing::error!(operation_id = key, fd = fd, error = ?e, "register_repoll: modify failed with error");
        // Some other error with modify
        return Some(Err(e));
      }
    } else {
      #[cfg(feature = "tracing")]
      tracing::debug!(
        operation_id = key,
        fd = fd,
        "register_repoll: modified poller successfully"
      );
    }

    #[cfg(feature = "tracing")]
    tracing::debug!(operation_id = key, fd = fd, "register_repoll: completed");

    Some(Ok(()))
  }

  pub(crate) fn detach(&self, key: u64) {
    #[cfg(feature = "tracing")]
    tracing::debug!(operation_id = key, "detach: starting");

    let reg = {
      let mut _lock = self.wakers.lock();
      #[cfg(feature = "tracing")]
      tracing::trace!(operation_id = key, "detach: acquired wakers lock");

      // Check if entry exists before removing
      let _entry_existed = _lock.contains_key(&key);
      let reg = _lock.remove(&key);

      // If we expected an entry but it's gone, this might indicate a double detach
      // However, it's also possible the background thread already processed it.
      // We'll be lenient here but log it.
      #[cfg(feature = "tracing")]
      if reg.is_some() {
        tracing::trace!(
          operation_id = key,
          "detach: removed entry from wakers"
        );
      } else if _entry_existed {
        tracing::warn!(
          operation_id = key,
          "detach: entry existed but was already removed (possible race condition)"
        );
      } else {
        tracing::trace!(
          operation_id = key,
          "detach: entry not found in wakers"
        );
      }
      reg
    };

    // Delete from poller without holding the wakers lock
    if let Some(mut reg) = reg {
      assert!(
        reg.fd > 0,
        "detach: invalid fd {} in registration for operation_id {}",
        reg.fd,
        key
      );

      if reg.has_waker() {
        #[cfg(feature = "tracing")]
        tracing::debug!(
          operation_id = key,
          fd = reg.fd,
          "detach: entry has waker, deleting from poller"
        );
        let delete_result = unsafe {
          use std::os::fd::BorrowedFd;

          // Remove from poller to prevent stale events
          self.poller.delete(&BorrowedFd::borrow_raw(reg.fd))
        };

        // If delete fails, it might mean the fd was already removed (e.g., by Oneshot mode)
        // This is OK, but we should log it
        if let Err(_e) = delete_result {
          #[cfg(feature = "tracing")]
          tracing::warn!(
            operation_id = key,
            fd = reg.fd,
            error = ?_e,
            "detach: failed to delete from poller (may already be removed by Oneshot mode)"
          );
          // Don't panic - this can happen legitimately with Oneshot mode
        } else {
          #[cfg(feature = "tracing")]
          tracing::debug!(
            operation_id = key,
            fd = reg.fd,
            "detach: deleted from poller"
          );
        }

        if let Some(waker) = reg.waker() {
          #[cfg(feature = "tracing")]
          tracing::trace!(operation_id = key, "detach: waking waker");
          waker.wake();
        } else {
          panic!(
            "detach: entry.has_waker() returned true but waker() returned None for operation_id {}",
            key
          );
        }
      } else {
        #[cfg(feature = "tracing")]
        tracing::debug!(
          operation_id = key,
          fd = reg.fd,
          "detach: entry has no waker, skipping poller delete"
        );
      }
    }
    // If registration doesn't exist, it was likely already processed and removed by background thread
    #[cfg(feature = "tracing")]
    tracing::debug!(operation_id = key, "detach: completed");
  }

  pub fn background(
    &'static self,
    mut sender: mpsc::Receiver<()>,
  ) -> thread::JoinHandle<()> {
    utils::create_worker(move || {
      #[cfg(feature = "tracing")]
      tracing::info!("background thread: started");
      let mut events = polling::Events::new();
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

        events.clear();

        #[cfg(feature = "tracing")]
        tracing::trace!("background thread: waiting on poller");

        let timeout = None;

        let wait_result = self.poller.wait(&mut events, timeout);

        if let Err(e) = wait_result {
          #[cfg(feature = "tracing")]
          tracing::error!(error = ?e, "background thread: poller.wait() failed");
          panic!(
            "background thread: poller.wait() failed with error: {:#?}",
            e
          );
        }

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
            operation_id = _operation_id,
            "background thread: processing event"
          );

          let waker = {
            #[cfg(feature = "tracing")]
            tracing::trace!(
              operation_id = operation_id,
              "background thread: acquiring wakers lock"
            );
            let mut _lock = self.wakers.lock();
            #[cfg(feature = "tracing")]
            tracing::trace!(
              operation_id = operation_id,
              "background thread: acquired wakers lock"
            );

            let entry = match _lock.get_mut(&operation_id) {
              Some(entry) => {
                #[cfg(feature = "tracing")]
                tracing::trace!(
                  operation_id = operation_id,
                  fd = entry.fd,
                  "background thread: found entry"
                );

                // Validate entry state
                assert!(
                  entry.fd > 0,
                  "background thread: invalid fd {} in entry for operation_id {}",
                  entry.fd,
                  operation_id
                );

                entry
              }
              None => {
                // Entry was removed (likely by detach()). This is OK - the operation completed
                // or was cancelled. Skip this event.
                #[cfg(feature = "tracing")]
                tracing::debug!(
                  operation_id = operation_id,
                  "background thread: entry not found (likely detached), skipping"
                );
                continue;
              }
            };

            // Take the waker but keep the entry in the map.
            // The entry will be updated by register_repoll() if the operation
            // needs to continue, or removed by detach() if it completes.
            match entry.waker() {
              Some(waker) => {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                  operation_id = operation_id,
                  "background thread: took waker from entry"
                );
                waker
              }
              None => {
                // Entry exists but no waker set yet. This can happen if:
                // 1. The event fired before register_repoll() was called (race condition)
                // 2. The waker was already taken by a previous event processing
                //
                // In case 1: The fd is ready, so the next poll will succeed. We should
                // re-register the fd in the poller so when register_repoll() is called,
                // it will immediately get the ready state.
                //
                // Actually, with Oneshot mode, the fd is automatically removed after
                // an event fires. So we need to re-add it so the next register_repoll()
                // will see it's ready.
                //
                // But wait - if the event already fired, the fd is ready NOW. The next
                // poll will succeed immediately. We don't need to do anything special.
                // Just skip this event - the operation will succeed on its next poll.
                #[cfg(feature = "tracing")]
                tracing::debug!(
                  operation_id = operation_id,
                  "background thread: entry has no waker (event fired before register_repoll or waker already taken). FD is ready, next poll will succeed."
                );
                continue;
              }
            }
          };

          #[cfg(feature = "tracing")]
          tracing::trace!(
            operation_id = operation_id,
            "background thread: released wakers lock, waking waker"
          );

          // Wake the waker without holding the lock.
          // Note: With PollMode::Oneshot, the poller automatically removes the fd
          // after an event fires, so we don't need to manually delete it.
          waker.wake();
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
      .stack_size(64 * 1024)
      .spawn(handle)
      .expect("failed to launch the worker thread")
  }
}

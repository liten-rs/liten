pub struct IoUring {
  inner: io_uring::IoUring,
  probe: io_uring::Probe,
  submission_guard: Mutex<()>,
}

impl IoUring {
  pub fn new() -> Self {
    let (io_uring, probe) = {
      let io_uring = io_uring::IoUring::new(256).unwrap();
      let mut probe = io_uring::Probe::new();
      io_uring.submitter().register_probe(&mut probe).unwrap();
      (io_uring, probe)
    };

    Self { inner: io_uring, probe, submission_guard: Mutex::new(()) }
  }
  pub fn from_i32_to_io_result(res: i32) -> std::io::Result<i32> {
    if res < 0 { Err(std::io::Error::from_raw_os_error(res)) } else { Ok(res) }
  }
}

impl IoBackend for IoUring {
  fn submit<T>(&self, op: T, store: &OpStore) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    if T::entry_supported(&self.probe) {
      let operation_id = store.next_id();
      let mut op = Box::new(op);
      let entry = op.create_entry().user_data(operation_id);

      // Insert the operation into wakers first
      store.insert(operation_id, op);

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
      OperationProgress::<T>::new_uring(operation_id)
    } else {
      todo!();
      // OperationProgress::<T>::new_blocking(op)
    }
  }

  fn notify(&self) {
    todo!();
  }

  fn tick(&self, store: &OpStore, can_wait: bool) {
    self.inner.submit_and_wait(if can_wait { 1 } else { 0 }).unwrap();

    // SAFETY: lio guarrantees that only one tick impl is running at any time.
    for io_entry in unsafe { self.inner.completion_shared() } {
      use std::mem;

      let operation_id = io_entry.user_data();

      // If the operation id is not registered (e.g., wake-up NOP), skip.
      let Some(set_done_result) = store.get_mut(operation_id, |entry| {
        entry.set_done(Self::from_i32_to_io_result(io_entry.result()))
      }) else {
        continue;
      };

      // if should keep.
      match set_done_result {
        None => {}
        Some(value) => match value {
          #[cfg(feature = "high")]
          ExtractedOpNotification::Waker(waker) => waker.wake(),
          ExtractedOpNotification::Callback(callback) => {
            store.get_mut(operation_id, |entry| callback.call(entry));
            assert!(store.remove(operation_id));
          }
        },
      }
    }
    unsafe { self.inner.completion_shared() }.sync();
  }
}

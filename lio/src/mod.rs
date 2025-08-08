// TODO: Safe shutdown
use std::{
  collections::HashMap,
  io,
  mem::{self, MaybeUninit},
  os::fd::RawFd,
  sync::OnceLock,
  task::Waker,
};

#[macro_use]
pub(crate) mod macros;

mod op;
mod op_progress;
mod op_registration;

use io_uring::IoUring;
use socket2::{SockAddr, SockAddrStorage};

use crate::{
  io::driver::{
    op_progress::OperationProgress,
    op_registration::{OpRegistration, OpRegistrationStatus},
  },
  loom::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
};

pub struct Driver(Arc<DriverInner>);
struct DriverInner {
  inner: IoUring,
  has_done_work: AtomicBool,

  submission_guard: Mutex<()>,
  wakers: Mutex<HashMap<u64, OpRegistration>>,
}

/// Public facing apis
impl Driver {
  impl_op!(op::Write, fn write(fd: RawFd, buf: Vec<u8>, offset: u64));
  impl_op!(op::Read, fn read(fd: RawFd, mem: Vec<u8>, offset: i64));
  impl_op!(op::Truncate,  fn truncate(fd: RawFd, len: u64));

  impl_op!(op::Socket,  fn socket(domain: socket2::Domain, ty: socket2::Type, proto: Option<socket2::Protocol>));

  // Seems to be a problem with this? value returned is always "called `Result::unwrap()` on an `Err` value: Os { code: -22, kind: Uncategorized, message: "Unknown error -22" }"
  impl_op!(op::Bind, fn bind(fd: RawFd, addr: socket2::SockAddr));

  impl_op!(op::Accept,  fn accept(fd: RawFd, addr: *mut MaybeUninit<SockAddrStorage>, len: *mut libc::socklen_t));
  impl_op!(op::Listen, fn listen(fd: RawFd, backlog: i32));
  impl_op!(op::Connect, fn connect(fd: RawFd, addr: SockAddr));

  impl_op!(op::Send, fn send(fd: RawFd, buf: Vec<u8>, flags: Option<i32>));
  impl_op!(op::Recv, fn recv(fd: RawFd, buf: Vec<u8>, flags: Option<i32>));
  impl_op!(op::Close, fn close(fd: RawFd));

  impl_op!(op::Tee, fn tee(fd_in: RawFd, fd_out: RawFd, size: u32));

  pub fn tick() {
    let driver = Driver::get();
    if driver
      .0
      .has_done_work
      .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
      .is_ok()
    {
      let _ = driver.0.inner.submit();
    }
  }

  pub fn background(&self) {
    let driver = self.0.clone();
    std::thread::spawn(move || {
      loop {
        driver.inner.submit_and_wait(1).unwrap();

        let entries: Vec<io_uring::cqueue::Entry> =
                // SAFETY: The only thread that is concerned with completion queue.
          unsafe { driver.inner.completion_shared() }.collect();

        for entry in entries {
          let operation_id = entry.user_data();

          let mut wakers = driver.wakers.lock().unwrap();

          let op_registration = wakers
            .get_mut(&operation_id)
            .expect("entry in completion queue doesnt exist in store.");

          let old_value = mem::replace(
            &mut op_registration.status,
            OpRegistrationStatus::Done { ret: entry.result() },
          );
          let waker: Option<Waker> = match old_value {
            OpRegistrationStatus::Waiting { ref registered_waker } => {
              registered_waker.take()
            }
            OpRegistrationStatus::Cancelling => {
              let reg = wakers.remove(&operation_id).unwrap();

              // Dropping the operation.
              (reg.drop_fn)(reg.op);

              None
            }
            OpRegistrationStatus::Done { .. } => {
              unreachable!("already processed entry");
            }
          };

          if let Some(waker) = waker {
            waker.wake();
          };
        }
        unsafe { driver.inner.completion_shared() }.sync();
      }
    });
  }
}

impl Driver {
  pub fn get() -> &'static Driver {
    static DRIVER: OnceLock<Driver> = OnceLock::new();

    DRIVER.get_or_init(|| {
      let driver = Driver(Arc::new(DriverInner {
        inner: IoUring::new(256).unwrap(),
        wakers: Mutex::new(HashMap::default()),
        submission_guard: Mutex::new(()),
        has_done_work: AtomicBool::new(false),
      }));

      driver.background();

      driver
    })
  }

  fn detatch(&self, id: u64) -> Option<()> {
    let mut _lock = Driver::get().0.wakers.lock().unwrap();
    let thing = _lock.get_mut(&id)?;

    thing.status = OpRegistrationStatus::Cancelling;

    Some(())
  }

  fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::AcqRel)
  }

  fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    let operation_id = Self::get().queue_submit::<T>(op);
    OperationProgress::<T>::new(operation_id)
  }

  fn queue_submit<T: op::Operation>(&self, op: T) -> u64 {
    let operation_id = Self::next_id();
    let entry = op.create_entry().user_data(operation_id);

    let mut _lock = self.0.wakers.lock().unwrap();

    // I think safe now?
    let _g = self.0.submission_guard.lock();
    unsafe {
      let mut sub = self.0.inner.submission_shared();
      sub.push(&entry).expect("unwrapping for now");
      sub.sync();
      drop(sub);
    }
    drop(_g);

    _lock.insert(operation_id, OpRegistration::new(op));

    self.0.has_done_work.store(true, Ordering::SeqCst);

    operation_id
  }

  pub fn check_registration<T: op::Operation>(
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
          Err(io::Error::from_raw_os_error(ret))
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

pub(crate) enum CheckRegistrationResult<V> {
  /// Waker has been registered and future should return Poll::Pending
  WakerSet,
  /// Value has been returned and future should poll anymore.
  Value(V),
}

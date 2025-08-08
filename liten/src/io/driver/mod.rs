// TODO: Safe shutdown
use std::{
  cell::{Cell, RefCell},
  collections::HashMap,
  future::Future,
  io,
  marker::PhantomData,
  mem::{self, MaybeUninit},
  os::fd::RawFd,
  pin::Pin,
  sync::{atomic::AtomicBool, Arc, OnceLock},
  task::{Context, Poll, Waker},
};

#[macro_use]
pub(crate) mod macros;

mod op;

use io_uring::IoUring;
use socket2::{SockAddr, SockAddrStorage};

use crate::loom::sync::{
  atomic::{AtomicU64, Ordering},
  Mutex,
};

pub struct OperationProgress<T> {
  id: OperationId,
  _m: PhantomData<T>,
}

impl<T> OperationProgress<T> {
  pub fn new(id: u64) -> Self {
    Self { id, _m: PhantomData }
  }
  pub fn detatch(self) {
    Driver::get().detatch(self.id);
  }
}

impl<T> Future for OperationProgress<T>
where
  T: op::Operation,
{
  type Output = T::Result;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let is_done = Driver::get()
      .check_registration::<T>(self.id, cx.waker().clone())
      .expect("Polled OperationProgress when not even registered");

    match is_done {
      CheckRegistrationResult::WakerSet => Poll::Pending,
      CheckRegistrationResult::Value(result) => Poll::Ready(result),
    }
  }
}

impl<T> Drop for OperationProgress<T> {
  fn drop(&mut self) {
    let mut _lock = Driver::get().0.wakers.lock().unwrap();
    if let Some(value) = _lock.get_mut(&self.id) {
      if let OpRegistrationStatus::Waiting { .. } = value.status {
        value.status = OpRegistrationStatus::Cancelling;
      }
    }
  }
}

pub struct Driver(Arc<DriverInner>);
struct DriverInner {
  inner: IoUring,
  has_done_work: AtomicBool,

  submission_guard: Mutex<()>,
  wakers: Mutex<HashMap<OperationId, OpRegistration>>,
}

type OperationId = u64;

pub struct OpRegistration {
  op: *const (),
  status: OpRegistrationStatus,
  drop_fn: fn(*const ()), // Function to properly drop the operation
}

impl OpRegistration {
  pub fn new<T>(op: T) -> Self {
    fn drop_op<T>(ptr: *const ()) {
      unsafe {
        let _ = Box::from_raw(ptr as *mut T);
      }
    }

    OpRegistration {
      op: Box::into_raw(Box::new(op)) as *const (),
      status: OpRegistrationStatus::Waiting {
        registered_waker: Cell::new(None),
      },
      drop_fn: drop_op::<T>,
    }
  }
}

impl std::fmt::Debug for OpRegistration {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OpRegistration")
      .field("op", &"*const ()")
      .field("status", &self.status)
      .field("drop_fn", &"fn(*const())")
      .finish()
  }
}
impl std::fmt::Debug for OpRegistrationStatus {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Waiting { registered_waker } => f
        .debug_struct("OpRegistrationStatus::Waiting")
        .field(
          "registered_waker (is some)",
          &unsafe { &*registered_waker.as_ptr() }.is_some(),
        )
        .finish(),
      Self::Cancelling => {
        f.debug_struct("OpRegistrationStatus::Cancelling").finish()
      }
      Self::Detatched => {
        f.debug_struct("OpRegistrationStatus::Detatched").finish()
      }
      Self::Done { ret } => {
        f.debug_struct("OpRegistrationStatus::Done").field("ret", &ret).finish()
      }
    }
  }
}

unsafe impl Send for OpRegistration {}
unsafe impl Sync for OpRegistration {}

pub enum OpRegistrationStatus {
  Waiting { registered_waker: Cell<Option<Waker>> },
  Cancelling,
  Done { ret: i32 },
  Detatched,
}

/// Public facing apis
impl Driver {
  impl_op!(op::Write, fn write(fd: RawFd, buf: Vec<u8>, offset: u64));
  impl_op!(op::Read, fn read(fd: RawFd, mem: Vec<u8>, offset: i64));
  impl_op!(op::Truncate,  fn truncate(fd: RawFd, len: u64));

  impl_op!(op::Socket,  fn socket(domain: i32, ty: i32, proto: i32));

  // Seems to be a problem with this? value returned is always "called `Result::unwrap()` on an `Err` value: Os { code: -22, kind: Uncategorized, message: "Unknown error -22" }"
  impl_op!(op::Bind, fn bind(fd: RawFd, addr: socket2::SockAddr));

  impl_op!(op::Accept,  fn accept(fd: RawFd, addr: *mut MaybeUninit<SockAddrStorage>, len: *mut libc::socklen_t));
  impl_op!(op::Listen, fn listen(fd: RawFd, backlog: i32));
  impl_op!(op::Connect, fn connect(fd: RawFd, addr: SockAddr));

  impl_op!(op::Send, fn send(fd: RawFd, buf: Vec<u8>, flags: Option<i32>));
  impl_op!(op::Recv, fn recv(fd: RawFd, len: u32, flags: Option<i32>));
  impl_op!(op::Close, fn close(fd: RawFd));

  impl_op!(op::Tee, fn tee(fd_in: RawFd, fd_out: RawFd, size: u32));

  pub fn maybe_submit(&self) {
    if self
      .0
      .has_done_work
      .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
      .is_ok()
    {
      self.0.inner.submit();
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
              if let Some(waker) = registered_waker.take() {
                Some(waker.clone())
              } else {
                None
              }
            }
            OpRegistrationStatus::Cancelling
            | OpRegistrationStatus::Detatched => {
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

  pub fn detatch(&self, id: u64) -> Option<()> {
    let mut _lock = self.0.wakers.lock().unwrap();
    let thing = _lock.get_mut(&id)?;

    thing.status = OpRegistrationStatus::Cancelling;

    Some(())
  }
  fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::AcqRel)
  }

  pub fn submit<T>(&self, op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    let operation_id = self.queue_submit::<T>(op);
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
      OpRegistrationStatus::Cancelling | OpRegistrationStatus::Detatched => {
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

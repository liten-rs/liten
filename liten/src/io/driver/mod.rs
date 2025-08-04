// TODO: Safe shutdown
use std::{
  cell::Cell,
  collections::HashMap,
  future::Future,
  io,
  marker::PhantomData,
  mem::{self, MaybeUninit},
  net::SocketAddr,
  os::fd::{BorrowedFd, RawFd},
  pin::Pin,
  sync::{Arc, OnceLock},
  task::{Context, Poll, Waker},
};
mod op;

use io_uring::IoUring;
use socket2::SockAddr;

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
}

impl<T> Future for OperationProgress<T>
where
  T: op::Operation,
{
  type Output = io::Result<(T::Output, i32)>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let is_done = Driver::get()
      .registration_is_done(self.id)
      .expect("Polled OperationProgress when not even registered");

    if is_done {
      Poll::Ready(Driver::get().registration_result::<T>(self.id))
    } else {
      // FIXME: Can get done between calling registration_is_done and
      // registration_register_waker
      Driver::get().registration_register_waker(self.id, cx.waker().clone());
      Poll::Pending
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
}

macro_rules! op_with_ret {
  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> ()) => {
    pub fn $name($($arg: $arg_ty),*) -> impl Future<Output = io::Result<()>> {
      let operation_progress = Driver::get().submit(<$operation>::new($($arg),*));
      async move {
        let (_, _) = operation_progress.await?;
        Ok(())
      }
    }
  };

  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> $return_alias:ident) => {
    pub fn $name($($arg: $arg_ty),*) -> impl Future<Output = io::Result<$return_alias>> {
      let operation_progress = Driver::get().submit(<$operation>::new($($arg),*));
      async move {
        let (_, ret) = operation_progress.await?;
        Ok(ret)
      }
    }
  };
  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* )) => {
    pub fn $name($($arg: $arg_ty),*) -> impl Future<Output = io::Result<i32>> {
      let operation_progress = Driver::get().submit(<$operation>::new($($arg),*));
      async move {
        let (_, ret) = operation_progress.await?;
        Ok(ret)
      }
    }
  };
}

macro_rules! op_with_value {
  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* )) => {
    pub fn $name($($arg: $arg_ty),*) -> impl Future<Output = io::Result<<$operation as op::Operation>::Output>> {
      let operation_progress = Driver::get().submit(<$operation>::new($($arg),*));
      async move {
        let (value, _) = operation_progress.await?;
        Ok(value)
      }
    }
  };

  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> both) => {
    pub fn $name($($arg: $arg_ty),*) -> impl Future<Output = io::Result<(<$operation as op::Operation>::Output, i32)>> {
      let operation_progress = Driver::get().submit(<$operation>::new($($arg),*));
      async move { operation_progress.await }
    }
  };
}

/// Public facing apis
impl Driver {
  op_with_ret!(op::Write, fn write(fd: RawFd, buf: Box<[u8]>, offset: u64));
  op_with_value!(op::Read, fn read(fd: RawFd, mem: Vec<u8>, offset: i64) -> both);

  op_with_ret!(op::Socket,  fn socket(domain: i32, ty: i32, proto: i32) -> RawFd);
  op_with_ret!(op::Bind, fn bind(fd: RawFd, addr: socket2::SockAddr) -> ());
  op_with_ret!(op::Accept,  fn accept(fd: RawFd, addr: *mut MaybeUninit<libc::sockaddr_storage>, len: *mut libc::socklen_t) -> RawFd);
  op_with_ret!(op::Listen, fn listen(fd: RawFd, backlog: i32) -> ());
  op_with_ret!(op::Connect, fn connect(fd: RawFd, addr: SocketAddr) -> RawFd);

  op_with_ret!(op::Send, fn send(fd: RawFd, buf: Vec<u8>, flags: Option<i32>));
  op_with_value!(op::Recv, fn recv(fd: RawFd, len: u32, flags: Option<i32>));
  op_with_ret!(op::Close, fn close(fd: RawFd) -> ());

  op_with_ret!(op::Tee, fn tee(fd_in: RawFd, fd_out: RawFd, size: u32));

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
      }));

      driver.background();

      driver
    })
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

    self.0.inner.submit().unwrap();
    operation_id
  }

  /// Returns:
  /// - None if not found
  /// - Some(true) if is done
  /// - Some(false) if not done
  pub fn registration_is_done(&self, id: u64) -> Option<bool> {
    let _lock = self.0.wakers.lock().unwrap();
    let registration = _lock.get(&id)?;
    Some(matches!(registration.status, OpRegistrationStatus::Done { .. }))
  }

  pub fn registration_result<T: op::Operation>(
    &self,
    id: u64,
  ) -> io::Result<(T::Output, i32)> {
    let mut _lock = self.0.wakers.lock().unwrap();
    let op_registration =
      _lock.get(&id).expect("op registration doesn't exist");

    let OpRegistrationStatus::Done { ret } = op_registration.status else {
      panic!("op registration is not done");
    };

    let op_registration = _lock.remove(&id).expect("what");

    // SAFETY: The pointer was created with Box::into_raw in queue_submit with a concrete type T
    // We can safely cast it back to the concrete type T
    let mut value = unsafe { Box::from_raw(op_registration.op as *mut T) };

    if ret < 0 {
      Err(io::Error::from_raw_os_error(ret))
    } else {
      Ok((value.result(), ret))
    }
  }

  pub fn registration_register_waker(
    &self,
    id: u64,
    waker: Waker,
  ) -> Option<()> {
    let mut _lock = self.0.wakers.lock().unwrap();
    let reg = _lock.get_mut(&id)?;

    match reg.status {
      OpRegistrationStatus::Waiting { ref mut registered_waker } => {
        registered_waker.replace(Some(waker));
      }
      _ => panic!("cannot register waker when not waiting status"),
    }
    Some(())
  }
}

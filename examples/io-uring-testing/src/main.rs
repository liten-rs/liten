#![allow(dead_code)]
use io_uring::IoUring;
use io_uring::squeue::Entry;
use io_uring::types::Fd;
use std::collections::HashMap;
use std::io::{self, Error};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub struct Write {
  fd: RawFd,
  buf: Box<[u8]>,
  offset: u64,
}

impl Write {
  pub fn new(fd: RawFd, buf: Box<[u8]>, offset: u64) -> Write {
    assert!((buf.len() as u32) <= u32::MAX);
    Self { fd, buf, offset }
  }
}

impl Operation for Write {
  type Output = ();
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    io_uring::opcode::Write::new(
      Fd(self.fd),
      self.buf.as_ptr(),
      self.buf.len() as u32,
    )
    .offset(self.offset)
    .build()
  }
  fn result(&mut self) -> Self::Output {
    ()
  }
}

// Things that implement this trait represent a command that can be executed using io-uring.
pub trait Operation {
  type Output;
  fn create_entry(&mut self) -> io_uring::squeue::Entry;
  // This is guarranteed after this has completed and only fire ONCE.
  fn result(&mut self) -> Self::Output;
}

pub struct Read {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  offset: u64,
}

impl Read {
  pub fn new(fd: i32, length: u32, offset: u64) -> Self {
    let mut mem = Vec::with_capacity(length as usize);

    for _ in 0..length as usize {
      mem.push(0);
    }
    Self { fd, buf: Some(mem), offset }
  }
}

impl Operation for Read {
  type Output = Vec<u8>;
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
    if let Some(ref mut buf) = self.buf {
      io_uring::opcode::Read::new(
        Fd(self.fd),
        buf.as_mut_ptr(),
        buf.len() as u32,
      )
      .offset(self.offset)
      .build()
    } else {
      unreachable!()
    }
  }
  fn result(&mut self) -> Self::Output {
    self.buf.take().expect("ran Read::result more than once.")
  }
}

type OperationId = u64;

#[derive(Clone)]
pub struct Driver(Arc<Mutex<DriverInner>>);

struct DriverInner {
  inner: IoUring,
  wakers: HashMap<OperationId, OpRegistration>,
}

pub struct OperationProgress<'a, T> {
  driver: Driver,
  id: OperationId,
  op: &'a mut T,
}

impl<'a, T> OperationProgress<'a, T> {
  pub fn new(driver: Driver, id: u64, op: &'a mut T) -> Self {
    Self { driver, id, op }
  }
}

impl<'a, T> Future for OperationProgress<'a, T>
where
  T: Operation,
{
  type Output = (Option<T::Output>, i32);

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let is_done = self
      .driver
      .registration_is_done(self.id)
      .expect("Polled OperationProgress when not even registered");

    if is_done {
      let num = self
        .driver
        .registration_result(self.id)
        .expect("Polled OperationProgress when not even registered");

      let result = (if num < 0 { None } else { Some(self.op.result()) }, num);
      Poll::Ready(result)
    } else {
      self.driver.registration_register_waker(self.id, cx.waker().clone());
      Poll::Pending
    }
  }
}

#[derive(Default)]
pub struct OpRegistration {
  waker_registered: Option<Waker>,
  ret: AtomicI32,
  is_done: AtomicBool,
}

impl OpRegistration {
  pub fn wake_registered(&self) {
    if let Some(ref waker) = self.waker_registered {
      waker.wake_by_ref();
    }
  }
}

impl Driver {
  fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::AcqRel)
  }
  pub fn new() -> Self {
    let inner = Driver(Arc::new(Mutex::new(DriverInner {
      inner: IoUring::new(256).unwrap(),
      wakers: HashMap::default(),
    })));
    inner
  }

  pub async fn write<T>(
    &self,
    fd: BorrowedFd<'_>,
    buf: &[u8],
    offset: u64,
  ) -> io::Result<i32>
  where
    T: AsRef<Path>,
  {
    let testing =
      self.submit(Write::new(fd.as_raw_fd(), buf.into(), offset)).await;

    match testing.0 {
      Some(_) => Ok(testing.1),
      None => Err(Error::last_os_error()),
    }
  }

  pub async fn read(
    &self,
    fd: BorrowedFd<'_>,
    length: u32,
    offset: u64,
  ) -> io::Result<Vec<u8>> {
    let (option_output, ret) =
      self.submit(Read::new(fd.as_raw_fd(), length, offset)).await;

    match option_output {
      Some(output) => Ok(output),
      None => Err(Error::from_raw_os_error(ret)),
    }
  }

  pub async fn submit<T>(&self, mut op: T) -> (Option<T::Output>, i32)
  where
    T: Operation,
  {
    let entry = op.create_entry();
    let operation_id = self.queue_submit(entry);

    OperationProgress::new(self.clone(), operation_id, &mut op).await
  }

  fn queue_submit(&self, entry: Entry) -> u64 {
    let operation_id = Self::next_id();
    let entry = entry.user_data(operation_id);

    let mut _lock = self.0.lock().unwrap();

    unsafe {
      _lock.inner.submission().push(&entry).expect("unwrapping for now");
    }

    _lock.wakers.insert(operation_id, OpRegistration::default());

    _lock.inner.submission().sync();
    _lock.inner.submit().unwrap();
    operation_id
  }

  pub fn poll_entries(&self) {
    let mut _lock = self.0.lock().unwrap();
    let iter = unsafe { _lock.inner.completion_shared() };
    for entry in iter {
      let operation_id = entry.user_data();

      if let Some(op_registration) = _lock.wakers.get(&operation_id) {
        op_registration.is_done.store(true, Ordering::SeqCst);
        op_registration.ret.store(entry.result(), Ordering::SeqCst);
        op_registration.wake_registered();
      }
    }
  }

  /// Returns:
  /// - None if not found
  /// - Some(true) if is done
  /// - Some(false) if not done
  pub fn registration_is_done(&self, id: u64) -> Option<bool> {
    let _lock = self.0.lock().unwrap();
    let registration = _lock.wakers.get(&id)?;
    Some(registration.is_done.load(Ordering::SeqCst))
  }

  pub fn registration_result(&self, id: u64) -> Option<i32> {
    let mut _lock = self.0.lock().unwrap();
    let testing = _lock.wakers.remove(&id)?;
    Some(testing.ret.load(Ordering::SeqCst))
  }

  pub fn registration_set_done(&self, id: u64) -> Option<()> {
    let _lock = self.0.lock().unwrap();
    let testing = _lock.wakers.get(&id)?;
    testing.is_done.store(true, Ordering::SeqCst);
    Some(())
  }

  pub fn registration_register_waker(
    &self,
    id: u64,
    waker: Waker,
  ) -> Option<()> {
    let mut _lock = self.0.lock().unwrap();
    let reg = _lock.wakers.get_mut(&id)?;
    reg.waker_registered.replace(waker);
    Some(())
  }
}

fn main() {}

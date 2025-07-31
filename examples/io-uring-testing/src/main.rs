#![allow(dead_code)]
use io_uring::IoUring;
use io_uring::squeue::Entry;
use io_uring::types::Fd;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::marker::PhantomData;
use std::mem;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

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
  type Output: Sized;
  fn create_entry(&mut self) -> io_uring::squeue::Entry;
  // This is guarranteed after this has completed and only fire ONCE.
  fn result(&mut self) -> Self::Output;
}

pub struct Read {
  fd: RawFd,
  buf: Option<Vec<u8>>,
  offset: i64,
}

impl Read {
  pub fn new(fd: i32, length: u32, offset: i64) -> Self {
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
      .offset(self.offset as u64)
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
  wakers: HashMap<OperationId, RefCell<OpRegistration>>,
}

pub struct OperationProgress<T> {
  driver: Driver,
  id: OperationId,
  _m: PhantomData<T>,
}

impl<T> OperationProgress<T> {
  pub fn new(driver: Driver, id: u64) -> Self {
    Self { driver, id, _m: PhantomData }
  }
}

impl<T> Future for OperationProgress<T>
where
  T: Operation,
{
  type Output = io::Result<(T::Output, i32)>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let is_done = self
      .driver
      .registration_is_done(self.id)
      .expect("Polled OperationProgress when not even registered");

    if is_done {
      Poll::Ready(self.driver.registration_result::<T>(self.id))
    } else {
      self.driver.registration_register_waker(self.id, cx.waker().clone());
      Poll::Pending
    }
  }
}

impl<T> Drop for OperationProgress<T> {
  fn drop(&mut self) {
    let _lock = self.driver.0.lock().unwrap();
    let value = _lock.wakers.get(&self.id).unwrap();
    value.borrow_mut().status = OpRegistrationStatus::Cancelling;
  }
}

pub struct OpRegistration {
  op: *const (),
  status: OpRegistrationStatus,
}

pub enum OpRegistrationStatus {
  Waiting { registered_waker: Cell<Option<Waker>> },
  Cancelling,
  Done { ret: i32 },
}

impl OpRegistration {
  pub fn wake_registered(&self) {
    if let OpRegistrationStatus::Waiting { ref registered_waker } = self.status
    {
      if let Some(waker) = registered_waker.take() {
        waker.wake_by_ref();
      }
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
    let (_, ret) =
      self.submit(Write::new(fd.as_raw_fd(), buf.into(), offset)).await?;

    Ok(ret)
  }

  pub async fn read(
    &self,
    fd: BorrowedFd<'_>,
    length: u32,
    offset: i64,
  ) -> io::Result<Vec<u8>> {
    let (buf, _) =
      self.submit(Read::new(fd.as_raw_fd(), length, offset)).await?;

    return Ok(buf);
  }

  pub async fn submit<T>(&self, mut op: T) -> io::Result<(T::Output, i32)>
  where
    T: Operation,
  {
    let entry = op.create_entry();
    let boxed = Box::new(op);

    let operation_id =
      self.queue_submit(entry, Box::into_raw(boxed) as *const _);

    OperationProgress::<T>::new(self.clone(), operation_id).await
  }

  fn queue_submit(&self, entry: Entry, op: *const ()) -> u64 {
    let operation_id = Self::next_id();
    let entry = entry.user_data(operation_id);

    let mut _lock = self.0.lock().unwrap();

    unsafe {
      _lock.inner.submission().push(&entry).expect("unwrapping for now");
    }

    _lock.wakers.insert(
      operation_id,
      RefCell::new(OpRegistration {
        op,
        status: OpRegistrationStatus::Waiting {
          registered_waker: Cell::new(None),
        },
      }),
    );

    _lock.inner.submission().sync();
    _lock.inner.submit().unwrap();
    operation_id
  }

  pub fn poll_entries(&self) {
    let mut _lock = self.0.lock().unwrap();
    let iter = unsafe { _lock.inner.completion_shared() };
    for entry in iter {
      let operation_id = entry.user_data();

      let op_registration = _lock
        .wakers
        .get(&operation_id)
        .expect("entry in completion queue doesnt exist in store.");

      let old_value = mem::replace(
        &mut op_registration.borrow_mut().status,
        OpRegistrationStatus::Done { ret: entry.result() },
      );
      let waker: Option<Waker> = match old_value {
        OpRegistrationStatus::Waiting { ref registered_waker } => {
          registered_waker.take().map(|x| x.clone())
        }
        OpRegistrationStatus::Cancelling => {
          let reg = _lock.wakers.remove(&operation_id).unwrap();
          let ptr = reg.borrow().op as *mut dyn Operation;

          let _ = Box::from_raw(ptr);
          Some(futures_task::noop_waker())
        }
        OpRegistrationStatus::Done { .. } => {
          unreachable!("already processed entry")
        }
      };

      if let Some(waker) = waker {
        waker.wake(); // Now check from future.
      };
    }
  }

  /// Returns:
  /// - None if not found
  /// - Some(true) if is done
  /// - Some(false) if not done
  pub fn registration_is_done(&self, id: u64) -> Option<bool> {
    let _lock = self.0.lock().unwrap();
    let registration = _lock.wakers.get(&id)?.borrow();
    Some(matches!(registration.status, OpRegistrationStatus::Done { .. }))
  }

  pub fn registration_result<T: Operation>(
    &self,
    id: u64,
  ) -> io::Result<(T::Output, i32)> {
    let mut _lock = self.0.lock().unwrap();
    let op_registration =
      _lock.wakers.remove(&id).expect("op registration doesn't exist");
    let op_registration = op_registration.borrow();

    let mut value = *unsafe { Box::from_raw(op_registration.op as *mut T) };

    let OpRegistrationStatus::Done { ret } = op_registration.status else {
      panic!("op registration is not done");
    };

    if ret < 0 {
      Err(io::Error::from_raw_os_error(ret))
    } else {
      Ok((value.result(), ret))
    }
  }

  // pub fn registration_set_done(&self, id: u64) -> Option<()> {
  //   let _lock = self.0.lock().unwrap();
  //   let testing = _lock.wakers.get(&id)?.borrow();
  //
  //       testing.status = OpRegistrationStatus::Done {};
  //
  //   // match testing.status {
  //   //   OpRegistrationStatus::Waiting {..} => {
  //   //   },
  //
  //   // waker_registered.replace(Some(waker));
  //   // }
  //   // _ => unreachable!(),
  //   // }
  //   // testing.is_done.store(true, Ordering::SeqCst);
  //   Some(())
  // }

  pub fn registration_register_waker(
    &self,
    id: u64,
    waker: Waker,
  ) -> Option<()> {
    let mut _lock = self.0.lock().unwrap();
    let mut reg = _lock.wakers.get(&id)?.borrow_mut();

    match reg.status {
      OpRegistrationStatus::Waiting { ref mut registered_waker } => {
        registered_waker.replace(Some(waker));
      }
      _ => panic!("cannot register waker when not waiting status"),
    }
    Some(())
  }
}

fn main() {
  let driver = Driver::new();

  let file = File::open("./README.md").unwrap();
  let mut fut = driver.read(file.as_fd(), 3, -1);

  driver.poll_entries();
  std::thread::sleep(Duration::from_secs(1));
  let result = unsafe { Pin::new_unchecked(&mut fut) }
    .poll(&mut Context::from_waker(&futures_task::noop_waker()));
  dbg!(&result);

  // let result = unsafe { Pin::new_unchecked(&mut fut) }
  //   .poll(&mut Context::from_waker(&futures_task::noop_waker()));

  // println!("{:#?}", String::from_utf8(result.));
}

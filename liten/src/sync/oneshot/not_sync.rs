use std::{
  cell::UnsafeCell,
  future::Future,
  mem::MaybeUninit,
  pin::Pin,
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  task::{Context, Poll, Waker},
};

use thiserror::Error;

use crate::sync::utils::has_flag;

const RECEIVER_DROPPED: u8 = 1 << 1;
const SENDER_DROPPED: u8 = 1 << 2;
const SENDER_SENT: u8 = 1 << 3;
const WAKER_REGISTERED: u8 = 1 << 4;

pub(crate) struct Channel<V> {
  state: AtomicU8,
  receiver_waker: UnsafeCell<MaybeUninit<Waker>>,
  value: UnsafeCell<MaybeUninit<V>>,
}

impl<V> Channel<V> {
  pub(crate) fn new() -> Self {
    Self {
      state: AtomicU8::new(0),
      receiver_waker: UnsafeCell::new(MaybeUninit::uninit()),
      value: UnsafeCell::new(MaybeUninit::uninit()),
    }
  }

  fn write_receiver_waker(&self, waker: Waker) {
    let waker_uninit = unsafe { self.receiver_waker.get().as_mut().unwrap() };
    waker_uninit.write(waker);
  }

  fn write_value(&self, value: V) {
    let waker_uninit = unsafe { self.value.get().as_mut().unwrap() };
    waker_uninit.write(value);
  }

  fn read_value_unchecked(&self) -> V {
    unsafe { (*self.value.get()).as_ptr().read() }
  }

  /// SAFETY: Caller should guarrantee waker is init'ed.
  fn wake_unchecked(&self) {
    let unsafecell_inner =
      unsafe { self.receiver_waker.get().as_ref() }.unwrap();
    let waker = unsafe { unsafecell_inner.assume_init_ref() };
    waker.wake_by_ref();
  }
}

pub struct Receiver<V> {
  channel: Arc<Channel<V>>,
}

#[derive(Error, Debug)]
pub enum ReceiverError {
  #[error("Sender has been dropped")]
  SenderDroppedError,
}

#[derive(Debug, Error)]
#[error("Sender has not been dropped")]
pub struct SenderStillAlive;

impl<V> Receiver<V> {
  pub(crate) fn new(channel: Arc<Channel<V>>) -> Self {
    Self { channel }
  }
  pub(crate) fn try_get_sender(&self) -> Result<Sender<V>, SenderStillAlive> {
    let value = self.channel.state.load(Ordering::Acquire);
    if !has_flag(value, SENDER_DROPPED) {
      // There is another receiver alive. This function cannot move forward.
      return Err(SenderStillAlive);
    };

    Ok(Sender { channel: self.channel.clone() })
  }
  pub fn try_recv(&self) -> Result<Option<V>, ReceiverError> {
    let state = self.channel.state.load(Ordering::Acquire);

    if has_flag(state, SENDER_SENT) {
      // SAFETY: If ChannelState::SENDER_SENT it's guarranteed for self.channel.value to be
      // initialised.
      return Ok(Some(self.channel.read_value_unchecked()));
    }

    if has_flag(state, SENDER_DROPPED) {
      return Err(ReceiverError::SenderDroppedError);
    }

    Ok(None)
  }
}

impl<V> Future for Receiver<V> {
  type Output = Result<V, ReceiverError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.try_recv() {
      Ok(value) => match value {
        Some(value) => Poll::Ready(Ok(value)),
        None => {
          self.channel.write_receiver_waker(cx.waker().clone());
          self.channel.state.fetch_or(WAKER_REGISTERED, Ordering::AcqRel);

          Poll::Pending
        }
      },
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

impl<V> Drop for Receiver<V> {
  fn drop(&mut self) {
    // This doesn't fail
    self.channel.state.fetch_or(RECEIVER_DROPPED, Ordering::AcqRel);
  }
}

#[derive(Clone)]
pub struct Sender<V> {
  channel: Arc<Channel<V>>,
}

#[derive(Debug, Error)]
pub enum SenderError {
  #[error("Receiver has been dropped")]
  ReceiverDroppedError,
}

impl<V> Sender<V> {
  pub(crate) fn new(channel: Arc<Channel<V>>) -> Self {
    Self { channel }
  }
  pub fn send(self, value: V) -> Result<(), SenderError> {
    let state = self.channel.state.load(Ordering::Acquire);

    if has_flag(state, RECEIVER_DROPPED) {
      return Err(SenderError::ReceiverDroppedError);
    }

    if has_flag(state, WAKER_REGISTERED) {
      // SAFETY: A waker is initialized because of the state.
      self.channel.wake_unchecked();
    }

    // This doesn't fail.
    self.channel.state.fetch_or(SENDER_SENT, Ordering::AcqRel);
    self.channel.write_value(value);

    Ok(())
  }
}

impl<V> Drop for Sender<V> {
  fn drop(&mut self) {
    // This doesn't fail
    let previous_value =
      self.channel.state.fetch_or(SENDER_DROPPED, Ordering::AcqRel);

    if has_flag(previous_value, WAKER_REGISTERED) {
      let unsafecell_inner =
        unsafe { self.channel.receiver_waker.get().as_ref() }.unwrap();
      let waker = unsafe { unsafecell_inner.assume_init_ref() };
      waker.wake_by_ref();
    }
  }
}

// All types in Channel are Send + Sync.
unsafe impl<V: Send> Send for Sender<V> {}
unsafe impl<V: Send> Send for Receiver<V> {}
unsafe impl<V: Sync> Sync for Sender<V> {}
unsafe impl<V: Sync> Sync for Receiver<V> {}

#[cfg(test)]
static_assertions::assert_impl_all!(Sender<()>: Send);
#[cfg(test)]
static_assertions::assert_impl_all!(Receiver<()>: Send);

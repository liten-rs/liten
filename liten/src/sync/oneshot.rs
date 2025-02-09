use std::{
  cell::UnsafeCell,
  error::Error,
  fmt::Display,
  future::{Future, IntoFuture},
  mem::MaybeUninit,
  pin::Pin,
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex,
  },
  task::{Context, Poll, Waker},
};

use crossbeam::atomic::AtomicCell;

bitflags::bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  struct ChannelState: u8 {
      const INITIALISED = 0;
      const RECEIVER_DROPPED = 1 << 1;
      const SENDER_DROPPED = 1 << 2;
      const SENDER_SENT = 1 << 3;
      const WAKER_REGISTERED = 1 << 4;
  }
}

// It's literally a u8
unsafe impl Send for ChannelState {}
unsafe impl Sync for ChannelState {}

pub struct Receiver<V> {
  channel: Arc<Channel<V>>,
}

impl<V> Drop for Receiver<V> {
  fn drop(&mut self) {
    // This doesn't fail
    let _ = self.channel.state.fetch_update(|mut old| {
      old.insert(ChannelState::RECEIVER_DROPPED);
      Some(old)
    });
  }
}

impl<V> Drop for Sender<V> {
  fn drop(&mut self) {
    let prev_value = self.channel.state.load();
    // This doesn't fail
    let value = self
      .channel
      .state
      .fetch_update(|mut old| {
        old.insert(ChannelState::SENDER_DROPPED);
        Some(old)
      })
      .unwrap();
    if value.contains(ChannelState::WAKER_REGISTERED) {
      let unsafecell_inner =
        unsafe { self.channel.waker.get().as_ref() }.unwrap();
      let waker = unsafe { unsafecell_inner.assume_init_ref() };
      waker.wake_by_ref();
    }
  }
}

#[derive(Clone)]
pub struct Sender<V> {
  channel: Arc<Channel<V>>,
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

pub struct Channel<V> {
  state: AtomicCell<ChannelState>,
  waker: UnsafeCell<MaybeUninit<Waker>>,
  value: UnsafeCell<MaybeUninit<V>>,
}

impl<V> Channel<V> {
  fn new() -> Self {
    Self {
      state: AtomicCell::new(ChannelState::INITIALISED),
      waker: UnsafeCell::new(MaybeUninit::uninit()),
      value: UnsafeCell::new(MaybeUninit::uninit()),
    }
  }

  fn write_waker(&self, waker: Waker) {
    let waker_uninit = unsafe { self.waker.get().as_mut().unwrap() };
    waker_uninit.write(waker);
  }

  fn write_value(&self, value: V) {
    let waker_uninit = unsafe { self.value.get().as_mut().unwrap() };
    waker_uninit.write(value);
  }

  fn read_value_unchecked(&self) -> V {
    unsafe { (*self.value.get()).as_ptr().read() }
  }

  fn wake_unchecked(&self) {
    // SAFETY: Caller should guarrantee waker is init'ed.
    let unsafecell_inner = unsafe { self.waker.get().as_ref() }.unwrap();
    let waker = unsafe { unsafecell_inner.assume_init_ref() };
    waker.wake_by_ref();
  }
}

/// A oneshot channel is a channel in which a value can only be sent once, and when sent the
/// sender is dropped. Simirlarly, The receiver can only receive data once, and is then dropped.
///
///
/// If a channel is guarranteed to send one piece of data, a number of optimisations can be made.
/// This makes oneshot channels very optimised for a async runtime.
pub fn channel<V>() -> (Sender<V>, Receiver<V>) {
  let channel = Arc::new(Channel::new());

  (Sender { channel: channel.clone() }, Receiver { channel: channel.clone() })
}

#[derive(Debug)]
pub struct ReceiverDroppedError;

impl Display for ReceiverDroppedError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("ReceiverDroppedError")
  }
}

impl Error for ReceiverDroppedError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }

  fn cause(&self) -> Option<&dyn Error> {
    None
  }

  fn description(&self) -> &str {
    "This channels receiver has been dropped"
  }
}

#[derive(Debug)]
pub struct SenderDroppedError;

impl Display for SenderDroppedError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("SenderDroppedError")
  }
}

impl Error for SenderDroppedError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }

  fn cause(&self) -> Option<&dyn Error> {
    None
  }

  fn description(&self) -> &str {
    "This channels sender has been dropped"
  }
}

impl<V> Sender<V> {
  pub fn send(self, value: V) -> Result<(), ReceiverDroppedError> {
    let state = self.channel.state.load();

    if state.contains(ChannelState::RECEIVER_DROPPED) {
      return Err(ReceiverDroppedError);
    }

    if state.contains(ChannelState::WAKER_REGISTERED) {
      // SAFETY: A waker is initialized because of the state.
      self.channel.wake_unchecked();
    }

    // This doesn't fail.
    let _ = self.channel.state.fetch_update(|mut previous| {
      previous.insert(ChannelState::SENDER_SENT);
      Some(previous)
    });

    self.channel.write_value(value);

    Ok(())
  }
}

impl<V> Future for Receiver<V> {
  type Output = Result<V, SenderDroppedError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let state = self.channel.state.load();

    if state.contains(ChannelState::SENDER_SENT) {
      // SAFETY: If ChannelState::SENDER_SENT it's guarranteed for self.channel.value to be
      // initialised.
      return Poll::Ready(Ok(self.channel.read_value_unchecked()));
    }

    if state.contains(ChannelState::SENDER_DROPPED) {
      return Poll::Ready(Err(SenderDroppedError));
    }
    self.channel.write_waker(cx.waker().clone());

    // This doesn't fail.
    let _ = self.channel.state.fetch_update(|mut previous| {
      previous.insert(ChannelState::WAKER_REGISTERED);
      Some(previous)
    });

    Poll::Pending
  }
}

#[test]
fn simple() {
  crate::runtime::Runtime::new().block_on(async move {
    let (sender, receiver) = channel();

    sender.send(());

    assert!(receiver.await.unwrap() == ());
  })
}

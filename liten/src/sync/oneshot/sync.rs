use std::{
  cell::UnsafeCell,
  future::Future,
  pin::Pin,
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex,
  },
  task::{Context, Poll, Waker},
};

use crate::sync::utils::has_flag;
use thiserror::Error;

/// Receiver has been dropped, this means that the sender cannot move forward and should return
/// an unrecoverable error.
const RECEIVER_DROPPED: u8 = 1 << 0;
/// Sender has been dropped, this means that the receiver cannot move forward and should return
/// error
const SENDER_DROPPED: u8 = 1 << 1;
//
///// Receiver listening for a value. This only gets set when the sender has not sent anything before
///// reciever is polled.
//const RECEIVER_LISTENING: u8 = 1 << 3;
//
const VALUE_TAKEN: u8 = 1 << 4;

/// Sender has set a value BEFORE receiver has started listening. Since in this scenario sender has
/// to wait. It's guarranteed that sender waker is set.
///
/// IF SENDER_SET_VALUE is set AND RECEIVER_LISTENING is NOT set, then sender should just return as
/// if successfull send. It means that receiver has already returned the sent value.
//const SENDER_SET_VALUE: u8 = 1 << 4;

pub(crate) struct SyncChannel<V> {
  state: AtomicU8,
  receiver_waker: Mutex<Option<Waker>>,
  sender_waker: Mutex<Option<Waker>>,
  value: Mutex<Option<V>>,
}

impl<V> SyncChannel<V> {
  pub(crate) fn new() -> Self {
    Self {
      state: AtomicU8::new(0),
      receiver_waker: Mutex::new(None),
      sender_waker: Mutex::new(None),
      value: Mutex::new(None),
    }
  }
  fn value_take(&self) -> Option<V> {
    let mut raw_ptr =
      self.value.lock().expect("expected value to be Some(...)");

    if raw_ptr.is_some() {
      self.state.fetch_or(VALUE_TAKEN, Ordering::AcqRel);
    }

    tracing::info!("SyncOneChannel: Value taken");

    raw_ptr.take()
  }

  /// Sets value and updates state accordingly.
  fn set_value(&self, value: V) {
    *self.value.lock().unwrap() = Some(value);
  }

  fn sender_waker(&self) -> Option<Waker> {
    let mut non_null_ptr = self.sender_waker.lock()
      .expect("SAFETY: the unwrap is on the option, not the value inside the option, this unwrap is okay");

    tracing::trace!(is_some = non_null_ptr.is_some(), "Fetching sender_waker");

    non_null_ptr.take()
  }
  fn set_sender_waker(&self, waker: Waker) {
    tracing::trace!("Setting sender_waker");
    *self.sender_waker.lock().unwrap() = Some(waker);
  }

  fn receiver_waker(&self) -> Option<Waker> {
    let mut non_null_ptr = self.receiver_waker.lock()
      .expect("SAFETY: the unwrap is on the option, not the value inside the option, this unwrap is okay");

    tracing::trace!(
      is_some = non_null_ptr.is_some(),
      "Fetching receiver_waker"
    );
    non_null_ptr.take()
  }

  fn set_receiver_waker(&self, waker: Waker) {
    tracing::trace!("Setting receiver_waker");
    *self.receiver_waker.lock().unwrap() = Some(waker);
  }
}
pub struct SyncReceiver<V> {
  channel: Arc<SyncChannel<V>>,
}

#[derive(Error, Debug, PartialEq)]
pub enum SyncReceiverError {
  #[error("Sender has been dropped")]
  SenderDroppedError,
}

#[derive(Debug, Error)]
#[error("Sender has not been dropped")]
pub struct SyncSenderStillAlive;

impl<V> SyncReceiver<V> {
  pub(crate) fn new(channel: Arc<SyncChannel<V>>) -> Self {
    Self { channel }
  }
}

impl<V> Future for SyncReceiver<V> {
  type Output = Result<V, SyncReceiverError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let state = self.channel.state.load(Ordering::Acquire);

    if let Some(value) = self.channel.value_take() {
      if let Some(sender_waker) = self.channel.sender_waker() {
        sender_waker.wake_by_ref();
      }

      tracing::trace!(
        "sync_oneshot_receiver: Value taken, has woken up sender and returning"
      );

      return Poll::Ready(Ok(value));
    }

    if has_flag(state, SENDER_DROPPED) {
      tracing::error!("sync_oneshot: sender has been dropped");
      return Poll::Ready(Err(SyncReceiverError::SenderDroppedError));
    };

    self.channel.set_receiver_waker(cx.waker().clone());
    tracing::trace!(
      "oneshot_sync_receiver: sender hasn't set, so Poll::Pending"
    );
    return Poll::Pending;
  }
}

impl<V> Drop for SyncReceiver<V> {
  fn drop(&mut self) {
    tracing::trace!("Dropping oneshot_sync_receiver");
    // Set the RECEIVER_DROPPED flag when receiver is dropped
    self.channel.state.fetch_or(RECEIVER_DROPPED, Ordering::Release);
  }
}
pub struct SyncSender<V> {
  channel: Arc<SyncChannel<V>>,
}

#[derive(Debug, Error, PartialEq)]
pub enum SyncSenderError {
  #[error("Receiver has been dropped")]
  ReceiverDroppedError,
}

impl<V> SyncSender<V> {
  pub(crate) fn new(channel: Arc<SyncChannel<V>>) -> Self {
    Self { channel }
  }
  pub fn send(self, value: V) -> SyncSenderSendFuture<V> {
    SyncSenderSendFuture {
      channel: self.channel.clone(),
      _value: UnsafeCell::new(value),
    }
  }
}

pub struct SyncSenderSendFuture<V> {
  channel: Arc<SyncChannel<V>>,
  _value: UnsafeCell<V>,
}

unsafe impl<V: Send> Send for SyncSenderSendFuture<V> {}
unsafe impl<V: Sync> Sync for SyncSenderSendFuture<V> {}

impl<V> Future for SyncSenderSendFuture<V> {
  type Output = Result<(), SyncSenderError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let state = self.channel.state.load(Ordering::Acquire);

    if has_flag(state, VALUE_TAKEN) {
      tracing::info!("oneshot_sync_sender: Receiver has set VALUE_TAKEN, exiting assuming receiver has value");
      return Poll::Ready(Ok(()));
    }

    if has_flag(state, RECEIVER_DROPPED) {
      return Poll::Ready(Err(SyncSenderError::ReceiverDroppedError));
    }

    self.channel.set_value(unsafe { self._value.get().read() });

    if let Some(value) = self.channel.receiver_waker() {
      value.wake();
    }

    self.channel.set_sender_waker(cx.waker().clone());

    tracing::trace!("oneshot_sync_sender: Sender is returning Poll::Pending");
    return Poll::Pending;
  }
}

impl<V> Drop for SyncSenderSendFuture<V> {
  fn drop(&mut self) {
    // Set the SENDER_DROPPED flag when sender is dropped
    self.channel.state.fetch_or(SENDER_DROPPED, Ordering::Release);
  }
}

impl<V> Drop for SyncSender<V> {
  fn drop(&mut self) {
    tracing::trace!("Dropping oneshot_sync_sender");
    // Set the SENDER_DROPPED flag when sender is dropped
    self.channel.state.fetch_or(SENDER_DROPPED, Ordering::Release);
  }
}

// All types in Channel are Send + Sync.
unsafe impl<V: Send> Send for SyncSender<V> {}
unsafe impl<V: Send> Send for SyncReceiver<V> {}
unsafe impl<V: Sync> Sync for SyncSender<V> {}
unsafe impl<V: Sync> Sync for SyncReceiver<V> {}

#[cfg(test)]
static_assertions::assert_impl_all!(SyncSender<()>: Send);
#[cfg(test)]
static_assertions::assert_impl_all!(SyncReceiver<()>: Send);

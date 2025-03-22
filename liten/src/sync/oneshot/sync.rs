use std::{
  cell::UnsafeCell,
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
};

use thiserror::Error;

pub struct Sender<V> {
  inner: Arc<Inner<V>>,
}

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Self { inner: arc_inner }
  }
  pub fn send(self, value: V) -> SenderSendFuture<V> {
    SenderSendFuture { inner: self.inner, parked_value: UnsafeCell::new(value) }
  }
}

pub struct SenderSendFuture<V> {
  inner: Arc<Inner<V>>,
  parked_value: UnsafeCell<V>,
}

impl<V> Future for SenderSendFuture<V> {
  type Output = Result<(), OneshotError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.inner.send_poll(cx, &self.parked_value)
  }
}

pub struct Receiver<V>(Arc<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Receiver(arc_inner)
  }
}

impl<V> Future for Receiver<V> {
  type Output = Result<V, OneshotError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.0.recv_poll(cx)
  }
}

const LISTENING: u8 = 1 << 0;
const SENT: u8 = 1 << 1;

pub struct State(u8);

pub enum StateV2<V> {
  Init,
  Listening(Waker),
  Sent(V, Option<Waker>),
}

pub struct Inner<V> {
  lock: Mutex<()>,
  state: UnsafeCell<StateV2<V>>,
}
unsafe impl<V: Send> Send for Inner<V> {}
unsafe impl<V: Sync> Sync for Inner<V> {}

impl<V> Inner<V> {
  pub(crate) fn new() -> Self {
    Inner { lock: Mutex::new(()), state: UnsafeCell::new(StateV2::Init) }
  }
  #[tracing::instrument(skip_all, name = "send_poll")]
  fn send_poll(
    &self,
    send_ctx: &mut Context<'_>,
    value: &UnsafeCell<V>,
  ) -> Poll<Result<(), OneshotError>> {
    let lock = self.lock.lock();
    tracing::trace!("locking");
    let state = unsafe { self.state.get().as_mut().unwrap() };

    let result = match *state {
      StateV2::Init => {
        tracing::trace!(waker = ?Some(&send_ctx.waker()), "setting sent");
        *state = StateV2::Sent(
          unsafe { value.get().read() },
          Some(send_ctx.waker().clone()),
        );
        Poll::Pending
      }
      StateV2::Sent(_, _) => unreachable!(),
      StateV2::Listening(ref waker) => {
        tracing::trace!(waker = ?None::<()>, "setting sent");
        let waker = waker.clone();

        *state = StateV2::Sent(unsafe { value.get().read() }, None);

        waker.wake();

        return Poll::Ready(Ok(()));
      }
    };
    tracing::trace!("release");
    drop(lock);

    return result;
  }

  #[tracing::instrument(skip_all, name = "recv_poll")]
  fn recv_poll(
    &self,
    recv_ctx: &mut Context<'_>,
  ) -> Poll<Result<V, OneshotError>> {
    let lock = self.lock.lock().unwrap();
    tracing::trace!("locking");
    let state = self.state.get();

    let result = match unsafe { state.read() } {
      StateV2::Init => {
        *unsafe { state.as_mut().unwrap() } =
          StateV2::Listening(recv_ctx.waker().clone());

        Poll::Pending
      }
      StateV2::Listening(_) => unreachable!(),
      StateV2::Sent(value, waker) => {
        if let Some(waker) = waker {
          waker.wake();
        }

        Poll::Ready(Ok(value))
      }
    };
    tracing::trace!("release");
    drop(lock);
    return result;
  }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum OneshotError {
  #[error("Channel has been dropped")]
  ChannelDropped,
}
///// Receiver has been dropped, this means that the sender cannot move forward and should return
///// an unrecoverable error.
//const RECEIVER_DROPPED: u8 = 1 << 0;
///// Sender has been dropped, this means that .send(self) has been called since its 'self' and not
///// '&self', this only tells half the story. This value coupled with SENDER_FUTURE_DROPPED needs to
///// be checked to get the full picture.
//const SENDER_DROPPED: u8 = 1 << 1;
//
///// Sender Future has been dropped, this means that the receiver cannot move forward and should return
///// error
//const SENDER_FUTURE_DROPPED: u8 = 1 << 2;
///// This is needed because if SENDER_DROPPED=1 and SENDER_FUTURE_DROPPED=0, it could mean that
///// `drop(sender)` has been called, or the [SyncSender]::call has been called. And different code paths
///// need to be taken based on what happened.
//const SENDER_FUTURE_INIT: u8 = 1 << 3;
////
/////// Receiver listening for a value. This only gets set when the sender has not sent anything before
/////// reciever is polled.
////const RECEIVER_LISTENING: u8 = 1 << 3;
////
//const VALUE_TAKEN: u8 = 1 << 4;

//struct SyncChannelInner<V> {
//  value: Option<V>,
//  sender_waker: Option<Waker>,
//  receiver_waker: Option<Waker>,
//  state: u8,
//}
//
//pub(crate) struct SyncChannel<V> {
//  //state: AtomicU8,
//  inner: Mutex<SyncChannelInner<V>>,
//}
//
//impl<V> SyncChannel<V> {
//  pub(crate) fn new() -> Self {
//    Self {
//      //state: AtomicU8::new(0),
//      inner: Mutex::new(SyncChannelInner {
//        state: 0,
//        value: None,
//        sender_waker: None,
//        receiver_waker: None,
//      }),
//    }
//  }
//
//  fn set_flag(&self, flag: u8) {
//    let mut _lock = self.inner.lock().unwrap();
//    _lock.state |= flag;
//    //self.state.fetch_or(flag, Ordering::SeqCst);
//  }
//
//  #[tracing::instrument("oneshot_sync_channel", skip_all)]
//  fn state(&self) -> u8 {
//    let value = self.inner.lock().unwrap().state;
//
//    tracing::trace!(state = value, "state snapshot");
//    value
//  }
//
//  #[tracing::instrument("oneshot_sync_channel", skip_all)]
//  fn value_take(&self) -> Option<V> {
//    let mut raw_ptr =
//      self.inner.lock().expect("expected value to be Some(...)");
//
//    if raw_ptr.value.is_some() {
//      self.set_flag(VALUE_TAKEN);
//    }
//
//    tracing::info!(is_some = raw_ptr.value.is_some(), "Value taken");
//
//    raw_ptr.value.take()
//  }
//
//  /// Sets value and updates state accordingly.
//  fn set_value(&self, value: V) {
//    let mut _lock = self.inner.lock()
//      .expect("SAFETY: the unwrap is on the option, not the value inside the option, this unwrap is okay");
//    _lock.value = Some(value);
//  }
//  #[tracing::instrument("oneshot_sync_channel", skip_all)]
//  fn sender_waker(&self) -> Option<Waker> {
//    let mut _lock = self.inner.lock()
//      .expect("SAFETY: the unwrap is on the option, not the value inside the option, this unwrap is okay");
//
//    tracing::trace!(
//      is_some = _lock.sender_waker.is_some(),
//      "Fetching sender_waker"
//    );
//
//    _lock.sender_waker.take()
//  }
//  #[tracing::instrument("oneshot_sync_channel", skip_all)]
//  fn set_sender_waker(&self, waker: Waker) {
//    tracing::trace!("Setting sender_waker");
//    let mut _lock = self.inner.lock().unwrap();
//    _lock.sender_waker = Some(waker);
//  }
//
//  #[tracing::instrument("oneshot_sync_channel", skip_all)]
//  fn receiver_waker(&self) -> Option<Waker> {
//    let mut _lock = self.inner.lock()
//      .expect("SAFETY: the unwrap is on the option, not the value inside the option, this unwrap is okay");
//
//    tracing::trace!(
//      is_some = _lock.receiver_waker.is_some(),
//      "Fetching receiver_waker"
//    );
//    _lock.receiver_waker.take()
//  }
//
//  #[tracing::instrument("oneshot_sync_channel", skip_all)]
//  fn set_receiver_waker(&self, waker: Waker) {
//    tracing::trace!("Setting receiver_waker");
//    let mut _lock = self.inner.lock().unwrap();
//    _lock.receiver_waker = Some(waker);
//  }
//}
//pub struct SyncReceiver<V> {
//  channel: Arc<SyncChannel<V>>,
//}
//
//#[derive(Error, Debug, PartialEq)]
//pub enum SyncReceiverError {
//  #[error("Sender has been dropped")]
//  SenderDroppedError,
//}
//
//#[derive(Debug, Error)]
//#[error("Sender has not been dropped")]
//pub struct SyncSenderStillAlive;
//
//impl<V> SyncReceiver<V> {
//  pub(crate) fn new(channel: Arc<SyncChannel<V>>) -> Self {
//    Self { channel }
//  }
//}
//
//impl<V> Future for SyncReceiver<V> {
//  type Output = Result<V, SyncReceiverError>;
//
//  #[tracing::instrument(name = "oneshot_sync_receiver_poll", skip_all)]
//  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//    tracing::trace!("running");
//    if let Some(value) = self.channel.value_take() {
//      if let Some(sender_waker) = self.channel.sender_waker() {
//        sender_waker.wake_by_ref();
//      }
//
//      tracing::trace!("Value taken, has woken up sender and returning");
//
//      return Poll::Ready(Ok(value));
//    }
//
//    // This runs in two scenarios:
//    // - If both SyncSender and Future returned by SyncSender::call is dropped.
//    // - If SyncSender is dropped BEFORE calling SyncSender::call
//    let state = self.channel.state();
//    if has_flag(state, SENDER_FUTURE_DROPPED)
//      || (!has_flag(state, SENDER_FUTURE_INIT)
//        && has_flag(state, SENDER_DROPPED))
//    {
//      tracing::error!("sender has been dropped");
//      return Poll::Ready(Err(SyncReceiverError::SenderDroppedError));
//    }
//
//    self.channel.set_receiver_waker(cx.waker().clone());
//    tracing::trace!("Poll::Pending");
//    return Poll::Pending;
//  }
//}
//
//impl<V> Drop for SyncReceiver<V> {
//  fn drop(&mut self) {
//    tracing::trace!("Dropping oneshot_sync_receiver");
//    // Set the RECEIVER_DROPPED flag when receiver is dropped
//    self.channel.set_flag(RECEIVER_DROPPED);
//  }
//}
//pub struct SyncSender<V> {
//  channel: Arc<SyncChannel<V>>,
//}
//
//#[derive(Debug, Error, PartialEq)]
//pub enum SyncSenderError {
//  #[error("Receiver has been dropped")]
//  ReceiverDroppedError,
//}
//
//impl<V> SyncSender<V> {
//  pub(crate) fn new(channel: Arc<SyncChannel<V>>) -> Self {
//    Self { channel }
//  }
//  pub fn send(self, value: V) -> SyncSenderSendFuture<V> {
//    self.channel.set_flag(SENDER_FUTURE_INIT);
//    SyncSenderSendFuture {
//      channel: self.channel.clone(),
//      _value: UnsafeCell::new(value),
//    }
//  }
//}
//
//pub struct SyncSenderSendFuture<V> {
//  channel: Arc<SyncChannel<V>>,
//  _value: UnsafeCell<V>,
//}
//
//unsafe impl<V: Send> Send for SyncSenderSendFuture<V> {}
//unsafe impl<V: Sync> Sync for SyncSenderSendFuture<V> {}
//
//impl<V> Future for SyncSenderSendFuture<V> {
//  type Output = Result<(), SyncSenderError>;
//
//  #[tracing::instrument(name = "oneshot_sync_sender_poll", skip_all)]
//  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//    let state = self.channel.state();
//
//    if has_flag(state, VALUE_TAKEN) {
//      tracing::info!(
//        "Receiver has set VALUE_TAKEN, exiting assuming receiver has value"
//      );
//      return Poll::Ready(Ok(()));
//    }
//
//    if has_flag(state, RECEIVER_DROPPED) {
//      tracing::trace!("receiver is dropped");
//      return Poll::Ready(Err(SyncSenderError::ReceiverDroppedError));
//    }
//
//    self.channel.set_value(unsafe { self._value.get().read() });
//
//    if let Some(value) = self.channel.receiver_waker() {
//      value.wake();
//    }
//
//    self.channel.set_sender_waker(cx.waker().clone());
//
//    tracing::trace!("Poll::Pending");
//    return Poll::Pending;
//  }
//}
//
//impl<V> Drop for SyncSenderSendFuture<V> {
//  fn drop(&mut self) {
//    tracing::trace!("Dropping oneshot_sync_sender_future");
//    // Set the SENDER_DROPPED flag when sender is dropped
//    self.channel.set_flag(SENDER_FUTURE_DROPPED);
//  }
//}
//
//impl<V> Drop for SyncSender<V> {
//  fn drop(&mut self) {
//    //tracing::trace!("Dropping oneshot_sync_sender");
//    // Set the SENDER_DROPPED flag when sender is dropped
//    self.channel.set_flag(SENDER_DROPPED);
//  }
//}
//
//// All types in Channel are Send + Sync.
//unsafe impl<V: Send> Send for SyncSender<V> {}
//unsafe impl<V: Send> Send for SyncReceiver<V> {}
//unsafe impl<V: Sync> Sync for SyncSender<V> {}
//unsafe impl<V: Sync> Sync for SyncReceiver<V> {}
//
//#[cfg(test)]
//static_assertions::assert_impl_all!(SyncSender<()>: Send);
//#[cfg(test)]
//static_assertions::assert_impl_all!(SyncReceiver<()>: Send);

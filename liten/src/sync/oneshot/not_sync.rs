use std::{
  future::Future,
  mem::{self, ManuallyDrop},
  pin::Pin,
  ptr,
  task::{Context, Poll, Waker},
};

use crate::loom::{
  cell::UnsafeCell,
  sync::{Arc, Mutex},
};

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum OneshotError {
  #[error("Channel has been dropped")]
  SenderDropped,
  #[error("Channel has not been dropped")]
  SenderNotDropped,

  #[error("Channel has been dropped")]
  ReceiverDropped,
}

// TODO: Get rid of Arc
pub struct Sender<V>(Arc<Inner<V>>);

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Self(arc_inner)
  }
  pub fn send(self, value: V) -> Result<(), OneshotError> {
    let this = ManuallyDrop::new(self);
    let inner = unsafe { Arc::from_raw(Arc::as_ptr(&this.0)) };

    inner.with_state(|state| match unsafe { &*state } {
      State::Init => {
        unsafe { ptr::write(state, State::Sent(value)) };
        Ok(())
      }
      State::SenderDropped => unreachable!(),
      State::ReceiverDropped => Err(OneshotError::ReceiverDropped),
      State::Sent(_) => unreachable!(),
      State::Listening(waker) => {
        unsafe { ptr::write(state, State::Sent(value)) };
        waker.wake_by_ref();
        Ok(())
      }
    })
  }
}

// This runs if not Sender::send has been called. If it has, then SenderSendFuture::drop does the
// job.
impl<V> Drop for Sender<V> {
  #[tracing::instrument(skip_all, name = "impl_drop_send")]
  fn drop(&mut self) {
    self.0.drop_channel_sender();
  }
}

//pub struct SenderSendFuture<V> {
//  inner: Arc<Inner<V>>,
//  value_to_send: *mut Option<V>,
//}
//unsafe impl<V: Send> Send for SenderSendFuture<V> {}

//impl<V> Future for SenderSendFuture<V> {
//  type Output = Result<(), OneshotError>;
//  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//    self.inner.send_poll(cx, self.value_to_send)
//  }
//}
//
//impl<V> Drop for SenderSendFuture<V> {
//  #[tracing::instrument(skip_all, name = "impl_drop_send_fut")]
//  fn drop(&mut self) {
//    self.inner.drop_channel();
//  }
//}

pub struct Receiver<V>(Arc<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Receiver(arc_inner)
  }

  pub fn try_recv(&self) -> Result<Option<V>, OneshotError> {
    self.0.try_recv()
  }

  pub fn try_get_sender(&self) -> Result<Sender<V>, OneshotError> {
    self.0.with_state(|state| match unsafe { &*state } {
      State::Init => Err(OneshotError::SenderNotDropped),
      State::SenderDropped => {
        let _ = unsafe { mem::replace(&mut *state, State::Init) };
        Ok(Sender(self.0.clone()))
      }
      State::Sent(_) => Err(OneshotError::SenderNotDropped),
      State::Listening(_) => Err(OneshotError::SenderNotDropped),
      State::ReceiverDropped => Err(OneshotError::SenderNotDropped),
    })
  }
}

impl<V> Drop for Receiver<V> {
  #[tracing::instrument(skip_all, name = "impl_drop_receiver")]
  fn drop(&mut self) {
    self.0.drop_channel_receiver();
  }
}

impl<V> Future for Receiver<V> {
  type Output = Result<V, OneshotError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.0.recv_poll(cx)
  }
}

pub enum State<V> {
  Init,
  Listening(Waker),
  Sent(V),
  SenderDropped,
  ReceiverDropped,
}

pub struct Inner<V>(Mutex<UnsafeCell<State<V>>>);

impl<V> Inner<V> {
  pub(super) fn new() -> Self {
    Inner(Mutex::new(UnsafeCell::new(State::Init)))
  }
  // SAFETY:
  //   - 'state': is behind mutex
  fn with_state<R>(&self, f: impl FnOnce(*mut State<V>) -> R) -> R {
    let lock = self.0.lock().unwrap();
    lock.with_mut(f)
  }

  //#[tracing::instrument(skip_all, name = "send_poll")]
  //fn send_poll(
  //  &self,
  //  send_ctx: &mut Context<'_>,
  //  value: *mut Option<V>,
  //) -> Poll<Result<(), OneshotError>> {
  //  // SAFETY:
  //  //   - 'value': rust guarantees that [Future::poll]'s only runs one at any time.
  //  self.with_state(|state| match unsafe { &*state } {
  //    State::ChannelDropped => Poll::Ready(Err(OneshotError::ChannelDropped)),
  //    State::Init => {
  //      assert!(unsafe { &*value }.is_some());
  //      let new_state = State::Sent(
  //        unsafe { &mut *value }
  //          .take()
  //          .expect("logic error: value already taken"),
  //        Some(send_ctx.waker().clone()),
  //      );
  //
  //      unsafe { ptr::write(state, new_state) };
  //      Poll::Pending
  //    }
  //    State::Sent(_) => {
  //      panic!("internal error: value already moved, cannot operate on value")
  //    }
  //    State::Listening(waker) => {
  //      let waker = waker.clone();
  //      let value = unsafe { &mut *value }
  //        .take()
  //        .expect("logic error: value already taken");
  //      unsafe { ptr::write(state, State::Sent(value, None)) };
  //
  //      waker.wake();
  //
  //      return Poll::Ready(Ok(()));
  //    }
  //  })
  //}

  //fn recv(&self) -> Option<Result<V, OneshotError>> {
  //  self.with_state(|state| {
  //    let old_state = mem::replace(
  //      unsafe { &mut *state },
  //      State::Listening(recv_ctx.waker().clone()),
  //    );
  //    match old_state {
  //      State::Init => {
  //        // Already replace Listening
  //        Poll::Pending
  //      }
  //      State::Sent(value) => Poll::Ready(Ok(value)),
  //      State::ChannelDropped => Poll::Ready(Err(OneshotError::ChannelDropped)),
  //      State::Listening(_) => unreachable!(),
  //    }
  //  })
  //}

  //  pub(crate) fn try_get_sender(&self) -> Result<Sender<V>, SenderStillAlive> {
  //    let value = self.channel.state();
  //    if !has_flag(value, SENDER_DROPPED) {
  //      // There is another receiver alive. This function cannot move forward.
  //      return Err(SenderStillAlive);
  //    };
  //
  //    Ok(Sender { channel: self.channel.clone() })
  //  }

  fn inner_try_recv(
    &self,
    state: *mut State<V>,
  ) -> Result<Option<V>, OneshotError> {
    let state = mem::replace(unsafe { &mut *state }, State::Init);
    match state {
      State::Init => Ok(None),
      State::ReceiverDropped => unreachable!(),
      State::SenderDropped => Err(OneshotError::SenderDropped),
      State::Listening(_) => unreachable!(),
      State::Sent(value) => Ok(Some(value)),
    }
  }
  pub fn try_recv(&self) -> Result<Option<V>, OneshotError> {
    self.with_state(|state| self.inner_try_recv(state))
  }

  #[tracing::instrument(skip_all, name = "recv_poll")]
  fn recv_poll(
    &self,
    recv_ctx: &mut Context<'_>,
  ) -> Poll<Result<V, OneshotError>> {
    self.with_state(|state| match self.inner_try_recv(state) {
      Err(error) => Poll::Ready(Err(error)),
      Ok(maybe_value) => match maybe_value {
        Some(value) => Poll::Ready(Ok(value)),
        None => {
          let new_state = State::Listening(recv_ctx.waker().clone());
          unsafe { mem::replace(&mut *state, new_state) };
          Poll::Pending
        }
      },
    })
  }

  fn drop_channel_sender(&self) {
    self.with_state(|state| unsafe { ptr::write(state, State::SenderDropped) })
  }

  fn drop_channel_receiver(&self) {
    self
      .with_state(|state| unsafe { ptr::write(state, State::ReceiverDropped) })
  }
}

//const RECEIVER_DROPPED: u8 = 1 << 1;
//const SENDER_DROPPED: u8 = 1 << 2;
//const SENDER_SENT: u8 = 1 << 3;
//const WAKER_REGISTERED: u8 = 1 << 4;
//
//struct InnerChannel<V> {
//  receiver_waker: MaybeUninit<Waker>,
//  value: MaybeUninit<V>,
//}
//
//pub(crate) struct Channel<V> {
//  state: AtomicU8,
//  inner: Mutex<InnerChannel<V>>,
//}
//
//impl<V> Channel<V> {
//  pub(crate) fn new() -> Self {
//    Self {
//      state: AtomicU8::new(0),
//      inner: Mutex::new(InnerChannel {
//        receiver_waker: MaybeUninit::uninit(),
//        value: MaybeUninit::uninit(),
//      }),
//    }
//  }
//
//  fn state(&self) -> u8 {
//    self.state.load(Ordering::SeqCst)
//  }
//
//  fn inner(&self) -> MutexGuard<'_, InnerChannel<V>> {
//    self.inner.lock().unwrap()
//  }
//
//  fn write_receiver_waker(&self, waker: Waker) {
//    let mut waker_uninit = self.inner();
//    waker_uninit.receiver_waker.write(waker);
//  }
//
//  fn write_value(&self, value: V) {
//    let mut waker_uninit = self.inner();
//    waker_uninit.value.write(value);
//  }
//
//  fn read_value_unchecked(&self) -> V {
//    let value = self.inner();
//    unsafe { value.value.as_ptr().read() }
//  }
//
//  /// SAFETY: Caller should guarrantee waker is init'ed.
//  fn wake_unchecked(&self) {
//    let ptr = self.inner();
//    let waker = unsafe { ptr.receiver_waker.assume_init_ref() };
//    waker.wake_by_ref();
//  }
//}
//
//pub struct Receiver<V> {
//  channel: Arc<Channel<V>>,
//}
//
//#[derive(Error, Debug, PartialEq, Eq)]
//pub enum ReceiverError {
//  #[error("Sender has been dropped")]
//  SenderDroppedError,
//}
//
//#[derive(Debug, Error)]
//#[error("Sender has not been dropped")]
//pub struct SenderStillAlive;
//
//impl<V> Receiver<V> {
//  pub(crate) fn new(channel: Arc<Channel<V>>) -> Self {
//    Self { channel }
//  }
//  pub(crate) fn try_get_sender(&self) -> Result<Sender<V>, SenderStillAlive> {
//    let value = self.channel.state();
//    if !has_flag(value, SENDER_DROPPED) {
//      // There is another receiver alive. This function cannot move forward.
//      return Err(SenderStillAlive);
//    };
//
//    Ok(Sender { channel: self.channel.clone() })
//  }
//  pub fn try_recv(&self) -> Result<Option<V>, ReceiverError> {
//    let state = self.channel.state();
//
//    if has_flag(state, SENDER_SENT) {
//      // SAFETY: If ChannelState::SENDER_SENT it's guarranteed for self.channel.value to be
//      // initialised.
//      return Ok(Some(self.channel.read_value_unchecked()));
//    }
//
//    if has_flag(state, SENDER_DROPPED) {
//      return Err(ReceiverError::SenderDroppedError);
//    }
//
//    Ok(None)
//  }
//}
//
//impl<V> Future for Receiver<V> {
//  type Output = Result<V, ReceiverError>;
//
//  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//    match self.try_recv() {
//      Ok(value) => match value {
//        Some(value) => Poll::Ready(Ok(value)),
//        None => {
//          self.channel.write_receiver_waker(cx.waker().clone());
//          self.channel.state.fetch_or(WAKER_REGISTERED, Ordering::SeqCst);
//
//          Poll::Pending
//        }
//      },
//      Err(err) => Poll::Ready(Err(err)),
//    }
//  }
//}
//
//impl<V> Drop for Receiver<V> {
//  fn drop(&mut self) {
//    // This doesn't fail
//    self.channel.state.fetch_or(RECEIVER_DROPPED, Ordering::SeqCst);
//  }
//}
//
//#[derive(Clone)]
//pub struct Sender<V> {
//  channel: Arc<Channel<V>>,
//}
//
//#[derive(Debug, Error)]
//pub enum SenderError {
//  #[error("Receiver has been dropped")]
//  ReceiverDroppedError,
//}
//
//impl<V> Sender<V> {
//  pub(crate) fn new(channel: Arc<Channel<V>>) -> Self {
//    Self { channel }
//  }
//  pub fn send(self, value: V) -> Result<(), SenderError> {
//    let state = self.channel.state();
//
//    if has_flag(state, RECEIVER_DROPPED) {
//      return Err(SenderError::ReceiverDroppedError);
//    }
//
//    if has_flag(state, WAKER_REGISTERED) {
//      // SAFETY: A waker is initialized because of the state.
//      self.channel.wake_unchecked();
//    }
//
//    // This doesn't fail.
//    self.channel.state.fetch_or(SENDER_SENT, Ordering::SeqCst);
//    self.channel.write_value(value);
//
//    Ok(())
//  }
//}
//
//impl<V> Drop for Sender<V> {
//  fn drop(&mut self) {
//    // This doesn't fail
//    let previous_value =
//      self.channel.state.fetch_or(SENDER_DROPPED, Ordering::SeqCst);
//
//    if has_flag(previous_value, WAKER_REGISTERED) {
//      let unsafecell_inner = self.channel.inner();
//      let waker = unsafe { unsafecell_inner.receiver_waker.assume_init_ref() };
//      waker.wake_by_ref();
//    }
//  }
//}
//
//// All types in Channel are Send + Sync.
//unsafe impl<V: Send> Send for Sender<V> {}
//unsafe impl<V: Send> Send for Receiver<V> {}
//unsafe impl<V: Sync> Sync for Sender<V> {}
//unsafe impl<V: Sync> Sync for Receiver<V> {}
//
//#[cfg(test)]
//static_assertions::assert_impl_all!(Sender<()>: Send);
//#[cfg(test)]
//static_assertions::assert_impl_all!(Receiver<()>: Send);

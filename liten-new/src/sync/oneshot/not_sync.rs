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
#[derive(Debug)]
pub struct Sender<V>(Arc<Inner<V>>);

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Self(arc_inner)
  }
  pub(crate) fn print_inner(&self) {
    self.0.with_state(|state| dbg!(unsafe { &*state }));
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

pub struct Receiver<V>(Arc<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Receiver(arc_inner)
  }

  pub(crate) fn print_inner(&self) {
    self.0.with_state(|state| dbg!(unsafe { &*state }));
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

impl<V> std::fmt::Debug for State<V> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Init => f.write_str("State::Init"),
      Self::SenderDropped => f.write_str("State::SenderDropped"),
      Self::ReceiverDropped => f.write_str("State::ReceiverDropped"),
      Self::Sent(_) => f.write_str("State::Sent(...)"),
      Self::Listening(waker) => {
        f.write_fmt(format_args!("State::Listening({:?})", waker))
      }
    }
  }
}

#[cfg_attr(debug_assertions, derive(Debug))]
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

  fn inner_try_recv(
    &self,
    state: *mut State<V>,
  ) -> Result<Option<V>, OneshotError> {
    match unsafe { state.read() } {
      State::Init => Ok(None),
      State::ReceiverDropped => unreachable!(),
      State::SenderDropped => Err(OneshotError::SenderDropped),
      State::Listening(_) => unreachable!(),
      State::Sent(value) => {
        let _ = mem::replace(unsafe { &mut *state }, State::Init);
        Ok(Some(value))
      }
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
          let _ = mem::replace(unsafe { &mut *state }, new_state);
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

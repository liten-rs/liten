use std::{
  fmt::Debug,
  future::Future,
  mem::ManuallyDrop,
  pin::Pin,
  task::{Context, Poll, Waker},
};

use crate::loom::sync::{Arc, Mutex, MutexGuard};

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

/// Sender for a oneshot channel.
///
/// This is the sender side of a oneshot channel. It can be used to send a value to the receiver.
///
/// # Example
///
/// ```rust
/// use liten::sync::oneshot;
///
/// #[liten::main]
/// async fn main() {
///   
///   let (sender, receiver) = oneshot::channel();
///   
///   sender.send(42);
///   
///   let value = receiver.await;
///   
///   assert_eq!(value, Ok(42));
/// }
/// ```
// TODO: Get rid of Arc
pub struct Sender<V>(Arc<Inner<V>>);

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Self(arc_inner)
  }
  pub fn send(self, value: V) -> Result<(), OneshotError> {
    let this = ManuallyDrop::new(self);
    let inner = unsafe { Arc::from_raw(Arc::as_ptr(&this.0)) };
    inner.send(value)
  }
}

// This runs if not Sender::send has been called. If it has, then SenderSendFuture::drop does the
// job.
impl<V> Drop for Sender<V> {
  fn drop(&mut self) {
    self.0.drop_channel_sender();
  }
}

/// Receiver for a oneshot channel.
///
/// This is the receiver side of a oneshot channel. It can be used to receive a value from the sender.
///
/// # Example
///
/// ```rust
/// use liten::sync::oneshot;
///
/// #[liten::main]
/// async fn main() {
///   let (sender, receiver) = oneshot::channel();
///   
///   sender.send(42);
///   
///   let value = receiver.await;
///   
///   assert_eq!(value, Ok(42));
/// }
/// ```
pub struct Receiver<V>(Arc<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Receiver(arc_inner)
  }

  pub fn try_recv(&self) -> Result<Option<V>, OneshotError> {
    self.0.try_recv()
  }

  pub fn try_get_sender(&self) -> Result<Sender<V>, OneshotError> {
    self.0.try_get_sender()?;
    Ok(Sender(self.0.clone()))
  }
}

impl<V> Drop for Receiver<V> {
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
  /// None is taken, and Some(V) is non-taken.
  Sent(Option<V>),
  SenderDropped,
  ReceiverDropped,
}

#[cfg(test)]
impl<V: PartialEq> PartialEq for State<V> {
  fn eq(&self, other: &Self) -> bool {
    match self {
      State::Init => matches!(other, State::Init),
      State::SenderDropped => matches!(other, State::SenderDropped),
      State::ReceiverDropped => matches!(other, State::ReceiverDropped),
      State::Listening(_) => {
        if let State::Listening(_) = other {
          true
        } else {
          false
        }
      }
      State::Sent(value1) => {
        if let State::Sent(value2) = other {
          value1 == value2
        } else {
          false
        }
      }
    }
  }
}

#[cfg(test)]
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

pub struct Inner<V>(Mutex<State<V>>);

impl<V> Inner<V> {
  pub(super) fn new() -> Self {
    Inner(Mutex::new(State::Init))
  }
}

impl<V> Inner<V> {
  fn try_recv(&self) -> Result<Option<V>, OneshotError> {
    self.inner_try_recv(&mut self.0.lock().unwrap())
  }
  fn recv_poll(
    &self,
    recv_ctx: &mut Context<'_>,
  ) -> Poll<Result<V, OneshotError>> {
    let mut state = self.0.lock().unwrap();
    match self.inner_try_recv(&mut state) {
      Err(error) => Poll::Ready(Err(error)),
      Ok(maybe_value) => match maybe_value {
        Some(value) => Poll::Ready(Ok(value)),
        None => {
          *state = State::Listening(recv_ctx.waker().clone());
          Poll::Pending
        }
      },
    }
  }
  fn send(&self, value: V) -> Result<(), OneshotError> {
    let state = &mut *self.0.lock().unwrap();
    match state {
      State::Init => {
        *state = State::Sent(Some(value));
        Ok(())
      }
      State::Listening(waker) => {
        let waker = waker.clone();
        *state = State::Sent(Some(value));
        waker.wake_by_ref();
        Ok(())
      }
      State::ReceiverDropped => Err(OneshotError::ReceiverDropped),
      State::SenderDropped | State::Sent(_) => unreachable!(),
    }
  }
  pub fn try_get_sender(&self) -> Result<(), OneshotError> {
    let mut state = self.0.lock().unwrap();
    match *state {
      State::Init => Err(OneshotError::SenderNotDropped),
      State::SenderDropped => {
        *state = State::Init;
        Ok(())
      }
      State::Sent(_) => Err(OneshotError::SenderNotDropped),
      State::Listening(_) => Err(OneshotError::SenderNotDropped),
      State::ReceiverDropped => Err(OneshotError::SenderNotDropped),
    }
  }
  fn drop_channel_sender(&self) {
    let mut state = self.0.lock().unwrap();
    *state = State::SenderDropped;
  }

  fn drop_channel_receiver(&self) {
    let mut state = self.0.lock().unwrap();
    *state = State::ReceiverDropped;
  }
}

impl<V> Inner<V> {
  fn inner_try_recv(
    &self,
    state: &mut MutexGuard<'_, State<V>>,
  ) -> Result<Option<V>, OneshotError> {
    match &mut **state {
      State::Init => Ok(None),
      State::ReceiverDropped => unreachable!(),
      State::SenderDropped => Err(OneshotError::SenderDropped),
      State::Listening(_) => unreachable!(
        "If State::Listening, inner_try_recv can't be called again"
      ),
      State::Sent(ref mut value) => match value.take() {
        Some(value) => Ok(Some(value)),
        None => {
          panic!("Value already taken: tried to run try_recv after value taken")
        }
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[crate::internal_test]
  fn test_inner_try_recv() {
    let inner = Inner::<u8>::new();

    assert_eq!(inner.try_recv(), Ok(None));
    inner.send(0).unwrap();
    let state = inner.0.lock().unwrap();
    assert_eq!(*state, State::Sent(Some(0)));
    drop(state);

    assert_eq!(inner.try_recv(), Ok(Some(0)));
    let state = inner.0.lock().unwrap();
    assert_eq!(*state, State::Sent(None));
    drop(state);
  }

  #[cfg(test)]
  mod tests {
    use crate::sync::oneshot::channel;

    #[crate::internal_test]
    fn channel_send_receive() {
      let (sender, receiver) = channel();
      sender.send(123).unwrap();
      assert_eq!(receiver.try_recv().unwrap(), Some(123));
    }

    #[crate::internal_test]
    fn drop_sender() {
      let (sender, receiver) = channel::<u32>();
      drop(sender);
      assert!(receiver.try_recv().is_err());
    }

    #[crate::internal_test]
    fn drop_receiver() {
      let (sender, receiver) = channel::<u32>();
      drop(receiver);
      assert!(sender.send(1).is_err());
    }
  }
}

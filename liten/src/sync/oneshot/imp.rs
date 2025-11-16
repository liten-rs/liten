use std::{
  cell::{RefCell, RefMut},
  fmt::Debug,
  future::Future,
  mem,
  pin::Pin,
  ptr::NonNull,
  task::{Context, Poll, Waker},
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

  #[error("try_recv called after taken value")]
  RecvAfterTakenValue,
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
pub struct Sender<V>(NonNull<Inner<V>>);

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: NonNull<Inner<V>>) -> Self {
    Self(arc_inner)
  }
  pub fn send(self, value: V) -> Result<(), OneshotError> {
    match unsafe { self.0.as_ref() }.send(value) {
      Ok(value) => {
        // So that Inner state doesn't get overridden by SenderDropped.
        mem::forget(self);

        Ok(value)
      }
      Err(err) => Err(err),
    }
  }
}

// This runs if not Sender::send has been called. If it has, then SenderSendFuture::drop does the
// job.
impl<V> Drop for Sender<V> {
  fn drop(&mut self) {
    if unsafe { self.0.as_ref() }.drop_channel_sender() {
      let _ = unsafe { Box::from_raw(self.0.as_ptr()) };
    }
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
pub struct Receiver<V>(NonNull<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: NonNull<Inner<V>>) -> Self {
    Receiver::<V>(arc_inner)
  }

  pub fn try_recv(&self) -> Result<Option<V>, OneshotError> {
    unsafe { self.0.as_ref() }.try_recv()
  }
}

impl<V> Drop for Receiver<V> {
  fn drop(&mut self) {
    if unsafe { self.0.as_ref() }.drop_channel_receiver() {
      let _ = unsafe { Box::from_raw(self.0.as_ptr()) };
    }
  }
}

impl<V> Future for Receiver<V> {
  type Output = Result<V, OneshotError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    unsafe { self.0.as_ref() }.recv_poll(cx)
  }
}

pub enum State<V> {
  Init,
  Listening(Waker),
  /// None is taken, and Some(V) is non-taken.
  Sent(Option<V>, Option<Waker>),
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
      State::Listening(_) => matches!(other, State::Listening(_)),
      State::Sent(value1, _waker) => {
        if let State::Sent(value2, _) = other {
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
      Self::Sent(_, _) => f.write_str("State::Sent(...)"),
      Self::Listening(waker) => {
        f.write_fmt(format_args!("State::Listening({waker:?})"))
      }
    }
  }
}

pub struct Inner<V>(RefCell<State<V>>);

impl<V> Inner<V> {
  pub(super) fn new() -> Self {
    Inner(RefCell::new(State::Init))
  }
}

impl<V> Inner<V> {
  fn try_recv(&self) -> Result<Option<V>, OneshotError> {
    self.inner_try_recv(&mut self.0.borrow_mut())
  }

  fn recv_poll(
    &self,
    recv_ctx: &mut Context<'_>,
  ) -> Poll<Result<V, OneshotError>> {
    let mut state = self.0.borrow_mut();

    match *state {
      State::Init => {
        *state = State::Listening(recv_ctx.waker().clone());
        Poll::Pending
      }
      State::ReceiverDropped => unreachable!(),
      State::SenderDropped => Poll::Ready(Err(OneshotError::SenderDropped)),
      State::Listening(_) => {
        *state = State::Listening(recv_ctx.waker().clone());
        Poll::Pending
      }
      State::Sent(ref mut value, ref mut waker) => match value.take() {
        Some(value) => {
          if let Some(waker) = waker.take() {
            waker.wake();
          };
          Poll::Ready(Ok(value))
        }
        None => {
          panic!("Value already taken: tried to run try_recv after value taken")
        }
      },
    }
  }

  fn send(&self, value: V) -> Result<(), OneshotError> {
    let state = &mut *self.0.borrow_mut();
    match state {
      State::Init => {
        *state = State::Sent(Some(value), None);
        Ok(())
      }
      State::Listening(waker) => {
        let waker = waker.clone();
        *state = State::Sent(Some(value), None);
        waker.wake_by_ref();
        Ok(())
      }
      State::ReceiverDropped => Err(OneshotError::ReceiverDropped),
      State::SenderDropped | State::Sent(_, _) => unreachable!(),
    }
  }
  // Returns if should drop
  fn drop_channel_sender(&self) -> bool {
    let mut state = self.0.borrow_mut();

    match *state {
      State::ReceiverDropped => true,
      _ => {
        *state = State::SenderDropped;
        false
      }
    }
  }

  fn drop_channel_receiver(&self) -> bool {
    let mut state = self.0.borrow_mut();

    match *state {
      State::SenderDropped | State::Sent(_, _) => true,
      _ => {
        *state = State::ReceiverDropped;
        false
      }
    }
  }
}

impl<V> Inner<V> {
  fn inner_try_recv(
    &self,
    state: &mut RefMut<'_, State<V>>,
  ) -> Result<Option<V>, OneshotError> {
    match &mut **state {
      State::Init => Ok(None),
      State::ReceiverDropped => unreachable!(),
      State::SenderDropped => Err(OneshotError::SenderDropped),
      State::Listening(_) => Ok(None),
      State::Sent(ref mut value, waker) => match value.take() {
        Some(value) => {
          if let Some(waker) = waker.take() {
            waker.wake();
          };

          Ok(Some(value))
        }
        None => Err(OneshotError::RecvAfterTakenValue),
      },
    }
  }
}

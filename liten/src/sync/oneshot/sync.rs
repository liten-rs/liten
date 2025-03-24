use std::{
  cell::UnsafeCell,
  future::Future,
  mem,
  pin::Pin,
  ptr,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
};

use thiserror::Error;

// TODO: Get rid of Arc
pub struct Sender<V>(Arc<Inner<V>>);

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Self(arc_inner)
  }
  pub fn send(self, value: V) -> SenderSendFuture<V> {
    let inner = self.0.clone();
    mem::forget(self);
    SenderSendFuture { inner, value_to_send: &value as *const V }
  }
}

impl<V> Drop for Sender<V> {
  fn drop(&mut self) {
    self.0.drop_channel()
  }
}

pub struct SenderSendFuture<V> {
  inner: Arc<Inner<V>>,
  value_to_send: *const V,
}
unsafe impl<V: Send> Send for SenderSendFuture<V> {}

impl<V> Future for SenderSendFuture<V> {
  type Output = Result<(), OneshotError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.inner.send_poll(cx, self.value_to_send)
  }
}

impl<V> Drop for SenderSendFuture<V> {
  fn drop(&mut self) {
    self.inner.drop_channel()
  }
}

pub struct Receiver<V>(Arc<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Receiver(arc_inner)
  }
}

impl<V> Drop for Receiver<V> {
  fn drop(&mut self) {
    self.0.drop_channel()
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
  Sent(V, Option<Waker>),
  Returned,
  ChannelDropped,
}

pub struct Inner<V>(Mutex<UnsafeCell<State<V>>>);

impl<V> Inner<V> {
  pub(super) fn new() -> Self {
    Inner(Mutex::new(UnsafeCell::new(State::Init)))
  }
  fn get_state(&self) -> *mut State<V> {
    let lock = self.0.lock().unwrap();
    lock.get()
  }
  #[tracing::instrument(skip_all, name = "send_poll")]
  fn send_poll(
    &self,
    send_ctx: &mut Context<'_>,
    value: *const V,
  ) -> Poll<Result<(), OneshotError>> {
    // SAFETY:
    //   - 'state': is behind mutex
    //   - 'value': rust guarantees that [Future::poll]'s only runs one at any time.
    let state = self.get_state();

    match unsafe { &*state } {
      State::ChannelDropped => Poll::Ready(Err(OneshotError::ChannelDropped)),
      State::Init => {
        tracing::trace!("entering StateV2::Init");
        let new_state = State::Sent(
          unsafe { ptr::read(value) },
          Some(send_ctx.waker().clone()),
        );

        unsafe { ptr::write(state, new_state) };
        Poll::Pending
      }
      State::Returned => Poll::Ready(Ok(())),
      State::Sent(_, _) => {
        panic!("logic error: value already moved, cannot operate on value")
      }
      State::Listening(ref waker) => {
        tracing::trace!("entering StateV2::Listening");

        let new_state = State::Sent(unsafe { std::ptr::read(value) }, None);
        unsafe { ptr::write(state, new_state) };

        waker.wake_by_ref();

        return Poll::Ready(Ok(()));
      }
    }
  }

  #[tracing::instrument(skip_all, name = "recv_poll")]
  fn recv_poll(
    &self,
    recv_ctx: &mut Context<'_>,
  ) -> Poll<Result<V, OneshotError>> {
    // SAFETY:
    //   - 'state': is behind mutex
    let state = self.get_state();
    match unsafe { ptr::read(state) } {
      State::ChannelDropped => Poll::Ready(Err(OneshotError::ChannelDropped)),
      State::Init => {
        let new_state = State::Listening(recv_ctx.waker().clone());
        unsafe { ptr::write(state, new_state) };

        Poll::Pending
      }
      State::Listening(_) => unreachable!(),
      State::Returned => unreachable!(),
      State::Sent(value, waker) => {
        unsafe { ptr::write(state, State::Returned) };
        if let Some(waker) = waker {
          waker.wake();
        }

        Poll::Ready(Ok(value))
      }
    }
  }

  fn drop_channel(&self) {
    let state = self.get_state();
    unsafe {
      ptr::write(state, State::ChannelDropped);
    }
  }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum OneshotError {
  #[error("Channel has been dropped")]
  ChannelDropped,
}

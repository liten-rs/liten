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
  ChannelDropped,
}

// TODO: Get rid of Arc
pub struct Sender<V>(Arc<Inner<V>>);

impl<V> Sender<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Self(arc_inner)
  }
  pub fn send(self, value: V) -> SenderSendFuture<V> {
    let this = ManuallyDrop::new(self);
    SenderSendFuture {
      inner: unsafe { Arc::from_raw(Arc::as_ptr(&this.0)) },
      value_to_send: Box::into_raw(Box::new(Some(value))),
    }
  }
}

// This runs if not Sender::send has been called. If it has, then SenderSendFuture::drop does the
// job.
impl<V> Drop for Sender<V> {
  #[tracing::instrument(skip_all, name = "impl_drop_send")]
  fn drop(&mut self) {
    self.0.drop_channel()
  }
}

pub struct SenderSendFuture<V> {
  inner: Arc<Inner<V>>,
  value_to_send: *mut Option<V>,
}
unsafe impl<V: Send> Send for SenderSendFuture<V> {}

impl<V> Future for SenderSendFuture<V> {
  type Output = Result<(), OneshotError>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.inner.send_poll(cx, self.value_to_send)
  }
}

impl<V> Drop for SenderSendFuture<V> {
  #[tracing::instrument(skip_all, name = "impl_drop_send_fut")]
  fn drop(&mut self) {
    drop(unsafe { Box::from_raw(self.value_to_send) });
    self.inner.drop_channel();
  }
}

pub struct Receiver<V>(Arc<Inner<V>>);

impl<V> Receiver<V> {
  pub(crate) fn new(arc_inner: Arc<Inner<V>>) -> Self {
    Receiver(arc_inner)
  }
}

impl<V> Drop for Receiver<V> {
  #[tracing::instrument(skip_all, name = "impl_drop_receiver")]
  fn drop(&mut self) {
    self.0.drop_channel();
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
  // SAFETY:
  //   - 'state': is behind mutex
  fn with_state<R>(&self, f: impl FnOnce(*mut State<V>) -> R) -> R {
    let lock = self.0.lock().unwrap();
    lock.with_mut(f)
  }

  #[tracing::instrument(skip_all, name = "send_poll")]
  fn send_poll(
    &self,
    send_ctx: &mut Context<'_>,
    value: *mut Option<V>,
  ) -> Poll<Result<(), OneshotError>> {
    // SAFETY:
    //   - 'value': rust guarantees that [Future::poll]'s only runs one at any time.
    self.with_state(|state| match unsafe { &*state } {
      State::ChannelDropped => Poll::Ready(Err(OneshotError::ChannelDropped)),
      State::Init => {
        assert!(unsafe { &*value }.is_some());
        let new_state = State::Sent(
          unsafe { &mut *value }
            .take()
            .expect("logic error: value already taken"),
          Some(send_ctx.waker().clone()),
        );

        unsafe { ptr::write(state, new_state) };
        Poll::Pending
      }
      State::Returned => {
        assert!(unsafe { &*value }.is_none());
        Poll::Ready(Ok(()))
      }
      State::Sent(_, _) => {
        panic!("internal error: value already moved, cannot operate on value")
      }
      State::Listening(waker) => {
        let waker = waker.clone();
        let value = unsafe { &mut *value }
          .take()
          .expect("logic error: value already taken");
        unsafe { ptr::write(state, State::Sent(value, None)) };

        waker.wake();

        return Poll::Ready(Ok(()));
      }
    })
  }

  #[tracing::instrument(skip_all, name = "recv_poll")]
  fn recv_poll(
    &self,
    recv_ctx: &mut Context<'_>,
  ) -> Poll<Result<V, OneshotError>> {
    self.with_state(|state| {
      // Can't use ptr::read here
      let old_state = mem::replace(
        unsafe { &mut *state },
        State::Listening(recv_ctx.waker().clone()),
      );

      match old_state {
        State::ChannelDropped => Poll::Ready(Err(OneshotError::ChannelDropped)),
        State::Init => Poll::Pending,
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
    })
  }

  fn drop_channel(&self) {
    self.with_state(|state| {
      match unsafe { &*state } {
        State::ChannelDropped => {}
        // Nothing has started.
        State::Init => unsafe { ptr::write(state, State::ChannelDropped) },
        // Not sure about this. Should maybe not happen
        State::Returned => unsafe { ptr::write(state, State::ChannelDropped) },
        State::Sent(_, waker) => match waker {
          // Sender has sent value and waiting for receiver. hence provided the waker.
          Some(waker) => {
            unsafe { ptr::write(state, State::ChannelDropped) };
            waker.wake_by_ref();
          }
          // Sender sent, whilst listening before.
          // We can't destroy channel inbetween sending and receiving if both is active
          None => {}
        },
        // Receiver listening without any value sent.
        State::Listening(waker) => {
          let waker_cloned = waker.clone();
          unsafe { ptr::write(state, State::ChannelDropped) };
          waker_cloned.wake_by_ref();
        }
      };
    })
  }
}

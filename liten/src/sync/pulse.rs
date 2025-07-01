use std::{
  cell::Cell,
  future::Future,
  sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
  },
  task::{Context, Poll, Waker},
};

use futures_task::noop_waker;
use thiserror::Error;

struct PulseReceiver(State);
struct PulseSender(State);

#[derive(Clone)]
struct State(Arc<StateInner>);

impl State {
  fn add_sender(&self) {
    self.0.num_senders.fetch_add(1, Ordering::AcqRel);
  }

  fn sub_sender(&self) {
    self.0.num_senders.fetch_sub(1, Ordering::AcqRel);
  }

  fn drop_receiver(&self) {
    if self
      .0
      .dropped_receiver
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
      .is_err()
    {
      panic!("receiver dropped twice");
    }
  }
  fn send(&self) -> Result<bool, ReceiverDropped> {
    if self.0.dropped_receiver.load(Ordering::Acquire) {
      return Err(ReceiverDropped);
    }

    self.0.pulse.store(true, Ordering::Relaxed);

    if let Some(waker) = self.0.waker.take() {
      waker.wake();
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn poll_wait(&self, cx: &Context) -> Poll<Result<(), SenderDropped>> {
    // Can't use weak here because if we do we don't know if it randomly fails or because value
    // is not true before.
    match self.0.pulse.compare_exchange(
      true,
      false,
      Ordering::AcqRel,
      Ordering::Relaxed,
    ) {
      Ok(_) => Poll::Ready(Ok(())),
      Err(_) => {
        if self.0.num_senders.load(Ordering::Acquire) == 0 {
          return Poll::Ready(Err(SenderDropped));
        }
        self.0.waker.set(Some(cx.waker().clone()));
        Poll::Pending
      }
    }
  }
}

struct StateInner {
  pulse: AtomicBool,
  waker: Cell<Option<Waker>>,
  num_senders: AtomicUsize,
  dropped_receiver: AtomicBool,
}

impl Default for State {
  fn default() -> Self {
    State(Arc::new(StateInner {
      pulse: AtomicBool::new(false),
      waker: Cell::new(None),
      dropped_receiver: AtomicBool::new(false),
      num_senders: AtomicUsize::new(0),
    }))
  }
}

pub fn pulse() -> (PulseSender, PulseReceiver) {
  let inner = State::default();
  (PulseSender::new(inner.clone()), PulseReceiver(inner))
}

impl PulseSender {
  fn new(inner: State) -> Self {
    inner.add_sender();
    Self(inner)
  }
  pub fn send(&self) -> Result<bool, ReceiverDropped> {
    self.0.send()
  }
}

impl Drop for PulseSender {
  fn drop(&mut self) {
    self.0.sub_sender();
  }
}

impl PulseReceiver {
  pub fn wait(&self) -> WaitFuture<'_> {
    WaitFuture(&self)
  }
}

impl Drop for PulseReceiver {
  fn drop(&mut self) {
    self.0.drop_receiver();
  }
}

pub struct WaitFuture<'a>(&'a PulseReceiver);

#[derive(Error, Debug)]
#[error("All Pulse::Sender's has been dropped")]
pub struct SenderDropped;
#[derive(Error, Debug)]
#[error("Pulse::Receiver has been dropped")]
pub struct ReceiverDropped;

impl Future for WaitFuture<'_> {
  type Output = Result<(), SenderDropped>;
  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    self.0 .0.poll_wait(cx)
  }
}

#[crate::internal_test]
fn test_pulse_creation() {
  let (sender, receiver) = pulse();
  assert!(sender.send().is_ok());
  assert!(futures_executor::block_on(receiver.wait()).is_ok());
}

#[crate::internal_test]
fn test_receiver_dropped() {
  let (sender, _) = pulse();
  assert!(matches!(sender.send(), Err(ReceiverDropped)));
}

#[crate::internal_test]
fn test_sender_dropped() {
  let (_sender, receiver) = pulse();
  let receiver_future = WaitFuture(&receiver);
  let waker = noop_waker();
  let mut cx = Context::from_waker(&waker);

  assert!(matches!(
    std::pin::pin!(receiver_future).as_mut().poll(&mut cx),
    Poll::Pending
  ));
}

#[crate::internal_test]
fn test_async_pulse() {
  crate::runtime::Runtime::builder().block_on(async {
    let (sender, receiver) = pulse();

    // Simulate async behavior
    let receiver_future = WaitFuture(&receiver);
    let _ = sender.send().unwrap();
    let result = receiver_future.await;

    assert!(result.is_ok());
  })
}

#[cfg(test)]
mod extra_tests {
  use super::*;
  use std::sync::Arc;
  use std::thread;

  #[crate::internal_test]
  fn multiple_sends() {
    let (s1, r) = pulse();
    assert!(s1.send().is_ok());
    assert!(s1.send().is_ok());
    assert!(futures_executor::block_on(r.wait()).is_ok());
  }

  #[crate::internal_test]
  fn poll_after_all_senders_dropped() {
    let (s, r) = pulse();
    drop(s);
    let result = futures_executor::block_on(r.wait());
    assert!(matches!(result, Err(_)));
  }
}

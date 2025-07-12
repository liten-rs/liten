use std::{
  cell::Cell,
  future::Future,
  sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
  },
  task::{Context, Poll, Waker},
};

use thiserror::Error;

pub struct PulseReceiver(Arc<State>);

unsafe impl Send for PulseReceiver {}
unsafe impl Sync for PulseReceiver {}

pub struct PulseSender(Arc<State>);

unsafe impl Send for PulseSender {}
unsafe impl Sync for PulseSender {}

impl State {
  fn add_sender(&self) {
    self.num_senders.fetch_add(1, Ordering::AcqRel);
  }

  fn sub_sender(&self) {
    self.num_senders.fetch_sub(1, Ordering::AcqRel);
  }

  fn drop_receiver(&self) {
    if self
      .dropped_receiver
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
      .is_err()
    {
      panic!("receiver dropped twice");
    }
  }
  fn send(&self) -> Result<bool, ReceiverDropped> {
    if self.dropped_receiver.load(Ordering::Acquire) {
      return Err(ReceiverDropped);
    }

    self.pulse.store(true, Ordering::Relaxed);

    if let Some(waker) = self.waker.take() {
      waker.wake();
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn poll_wait(&self, cx: &Context) -> Poll<Result<(), SenderDropped>> {
    // Can't use weak here because if we do we don't know if it randomly fails or because value
    // is not true before.
    match self.pulse.compare_exchange(
      true,
      false,
      Ordering::AcqRel,
      Ordering::Relaxed,
    ) {
      Ok(_) => Poll::Ready(Ok(())),
      Err(_) => {
        if self.num_senders.load(Ordering::Acquire) == 0 {
          return Poll::Ready(Err(SenderDropped));
        }
        self.waker.set(Some(cx.waker().clone()));
        Poll::Pending
      }
    }
  }
}

pub(crate) struct State {
  pulse: AtomicBool,
  waker: Cell<Option<Waker>>,
  num_senders: AtomicUsize,
  dropped_receiver: AtomicBool,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl Default for State {
  fn default() -> Self {
    State {
      pulse: AtomicBool::new(false),
      waker: Cell::new(None),
      dropped_receiver: AtomicBool::new(false),
      num_senders: AtomicUsize::new(0),
    }
  }
}

impl PulseSender {
  pub(super) fn new(inner: Arc<State>) -> Self {
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
  pub(super) fn new(inner: Arc<State>) -> Self {
    Self(inner)
  }
  pub fn wait(&self) -> WaitFuture<'_> {
    WaitFuture(self)
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::sync::pulse::pulse;
  pub use crate::testing_util::noop_waker;

  #[crate::internal_test]
  fn test_pulse_creation() {
    let (sender, receiver) = pulse();
    assert!(sender.send().is_ok());
    assert!(crate::future::block_on(receiver.wait()).is_ok());
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
    crate::runtime::Runtime::single_threaded().block_on(async {
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

    #[crate::internal_test]
    fn multiple_sends() {
      let (s1, r) = pulse();
      assert!(s1.send().is_ok());
      assert!(s1.send().is_ok());
      assert!(crate::future::block_on(r.wait()).is_ok());
    }

    #[crate::internal_test]
    fn poll_after_all_senders_dropped() {
      let (s, r) = pulse();
      drop(s);
      let result = crate::future::block_on(r.wait());
      assert!(result.is_err());
    }
  }
}

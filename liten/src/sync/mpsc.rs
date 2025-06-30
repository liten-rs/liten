// use super::utils::has_flag;
// use futures_core::{FusedFuture, Stream};
// use std::{
//   collections::VecDeque,
//   future::Future,
//   task::{Context, Poll, Waker},
// };
//
// use crate::loom::sync::{
//   atomic::{AtomicU16, AtomicU8, Ordering},
//   Arc, Mutex,
// };
//
// const INITIALISED: u8 = 0;
// const RECEIVER_DROPPED: u8 = 1 << 1;

pub use async_channel::*;

// pub fn unbounded<T>() -> (async_channel::Sender<T>, async_channel::Receiver<T>)
// {
//   async_channel::unbounded()
// }
//
// pub fn bounded<T>(
//   num: usize,
// ) -> (async_channel::Sender<T>, async_channel::Receiver<T>) {
//   async_channel::bounded(num)
// }
//
// pub struct UnboundedChannel<T> {
//   state: AtomicU8,
//   num_senders: AtomicU16,
//   data: Mutex<ChannelState<T>>,
// }
//
// pub struct ChannelState<T> {
//   waker: Option<Waker>,
//   list: VecDeque<T>,
// }
//
// impl<T> UnboundedChannel<T> {
//   fn state_drop_receiver(&self) {
//     self.state.fetch_or(RECEIVER_DROPPED, Ordering::AcqRel);
//   }
//
//   fn state_has_receiver_dropped(&self) -> bool {
//     has_flag(self.state.load(Ordering::Acquire), RECEIVER_DROPPED)
//   }
//
//   fn senders_has_all_dropped(&self) -> bool {
//     self.num_senders.load(Ordering::Acquire) == 0
//   }
//
//   fn senders_add_sender(&self) {
//     self.num_senders.fetch_add(1, Ordering::AcqRel);
//   }
//
//   fn senders_sub_sender(&self) {
//     self.num_senders.fetch_sub(1, Ordering::AcqRel);
//   }
// }
//
// impl<T> Default for UnboundedChannel<T> {
//   fn default() -> Self {
//     Self {
//       data: Mutex::new(ChannelState {
//         waker: None,
//         list: VecDeque::with_capacity(512),
//       }),
//       state: AtomicU8::new(INITIALISED),
//       num_senders: AtomicU16::new(0),
//     }
//   }
// }
//
// impl<T> UnboundedChannel<T> {
//   fn with_capacity(capacity: usize) -> Self {
//     Self {
//       data: Mutex::new(ChannelState {
//         waker: None,
//         list: VecDeque::with_capacity(capacity),
//       }),
//       ..Default::default()
//     }
//   }
//
//   fn send(&self, t: T) -> Result<(), ReceiverDroppedError> {
//     println!("Send");
//     if self.state_has_receiver_dropped() {
//       return Err(ReceiverDroppedError);
//     }
//
//     let mut lock = self.data.lock().unwrap();
//     lock.list.push_back(t);
//
//     if let Some(tesing) = lock.waker.as_ref() {
//       tesing.wake_by_ref();
//     }
//
//     Ok(())
//   }
//
//   fn try_recv(&self) -> Result<T, RecvError> {
//     if self.senders_has_all_dropped() {
//       return Err(RecvError::Disconnected);
//     }
//
//     let mut lock = self.data.lock().unwrap();
//     match lock.list.pop_front() {
//       Some(t) => Ok(t),
//       None => Err(RecvError::Empty),
//     }
//   }
//   fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<Result<T, RecvError>> {
//     match self.try_recv() {
//       Ok(value) => {
//         println!("Recv");
//         Poll::Ready(Ok(value))
//       }
//       Err(err) => match err {
//         RecvError::Disconnected => Poll::Ready(Err(RecvError::Disconnected)),
//         RecvError::Empty => {
//           let mut lock = self.data.lock().unwrap();
//           lock.waker = Some(cx.waker().clone());
//
//           Poll::Pending
//         }
//       },
//     }
//   }
// }
//
// pub struct Receiver<T> {
//   channel: Arc<UnboundedChannel<T>>,
// }
//
// #[derive(Debug, PartialEq)]
// pub enum RecvError {
//   Disconnected,
//   Empty,
// }
//
// impl<T> From<Arc<UnboundedChannel<T>>> for Receiver<T> {
//   fn from(channel: Arc<UnboundedChannel<T>>) -> Self {
//     Self { channel }
//   }
// }
//
// impl<T> Drop for Receiver<T> {
//   fn drop(&mut self) {
//     self.channel.state_drop_receiver();
//   }
// }
//
// pub struct ReceiverIter<'a, T>(&'a Receiver<T>);
//
// impl<T> Iterator for ReceiverIter<'_, T> {
//   type Item = T;
//
//   fn next(&mut self) -> Option<Self::Item> {
//     self.0.try_recv().ok()
//   }
// }
//
// impl<T> Receiver<T> {
//   pub fn try_iter(&self) -> ReceiverIter<'_, T> {
//     ReceiverIter(self)
//   }
// }
//
// pub struct ReceiverFuture<'a, T>(&'a Receiver<T>);
//
// impl<T> Future for ReceiverFuture<'_, T> {
//   type Output = Result<T, RecvError>;
//
//   fn poll(
//     self: std::pin::Pin<&mut Self>,
//     cx: &mut std::task::Context<'_>,
//   ) -> std::task::Poll<Self::Output> {
//     self.0.channel.poll_recv(cx)
//   }
// }
//
// impl<T> FusedFuture for ReceiverFuture<'_, T> {
//   fn is_terminated(&self) -> bool {
//     self.0.channel.state_has_receiver_dropped()
//   }
// }
//
// impl<T> Receiver<T> {
//   pub async fn recv(&self) -> Result<T, RecvError> {
//     ReceiverFuture(self).await
//   }
//
//   pub fn try_recv(&self) -> Result<T, RecvError> {
//     self.channel.try_recv()
//   }
// }
// impl<T> Stream for Receiver<T> {
//   type Item = T;
//   fn poll_next(
//     self: std::pin::Pin<&mut Self>,
//     cx: &mut std::task::Context<'_>,
//   ) -> Poll<Option<Self::Item>> {
//     let pinn = std::pin::pin!(self.recv());
//     match pinn.poll(cx) {
//       Poll::Ready(value) => match value {
//         Ok(value) => Poll::Ready(Some(value)),
//         Err(err) => match err {
//           RecvError::Disconnected => Poll::Ready(None),
//           RecvError::Empty => Poll::Pending,
//         },
//       },
//       Poll::Pending => Poll::Pending,
//     }
//   }
// }
//
// #[derive(Debug, PartialEq)]
// pub struct ReceiverDroppedError;
//
// pub struct Sender<T> {
//   channel: Arc<UnboundedChannel<T>>,
// }
//
// impl<T> From<Arc<UnboundedChannel<T>>> for Sender<T> {
//   fn from(channel: Arc<UnboundedChannel<T>>) -> Self {
//     channel.senders_add_sender();
//     Self { channel }
//   }
// }
//
// impl<T> Sender<T> {
//   pub fn send(&self, t: T) -> Result<(), ReceiverDroppedError> {
//     self.channel.send(t)
//   }
// }
//
// impl<T> Clone for Sender<T> {
//   fn clone(&self) -> Self {
//     self.channel.senders_add_sender();
//     Sender { channel: self.channel.clone() }
//   }
// }
//
// impl<T> Drop for Sender<T> {
//   fn drop(&mut self) {
//     self.channel.senders_sub_sender();
//   }
// }
//
// #[cfg(test)]
// mod tests {
//
//   use super::*;
//   use std::sync::Arc as StdArc;
//   use std::task::{Context, Poll, Waker};
//
//   #[test]
//   fn test_channel_creation() {
//     let (sender, receiver) = unbounded::<u32>();
//     assert!(sender.send(1).is_ok());
//     assert!(receiver.try_recv().is_ok());
//
//     let (sender, receiver) = channel::<u32>(10);
//     assert!(sender.send(1).is_ok());
//     assert!(receiver.try_recv().is_ok());
//   }
//
//   #[test]
//   fn test_send_and_receive() {
//     let (sender, receiver) = unbounded::<u32>();
//     assert!(sender.send(1).is_ok());
//     assert_eq!(receiver.try_recv(), Ok(1));
//   }
//
//   #[test]
//   fn test_try_recv() {
//     let (sender, receiver) = unbounded::<u32>();
//     assert!(sender.send(1).is_ok());
//     assert_eq!(receiver.try_recv(), Ok(1));
//     assert_eq!(receiver.try_recv(), Err(RecvError::Empty));
//   }
//
//   #[test]
//   fn test_poll_recv() {
//     let (sender, receiver) = unbounded::<u32>();
//     let waker = noop_waker();
//     let mut cx = Context::from_waker(&waker);
//
//     assert!(sender.send(1).is_ok());
//     assert_eq!(receiver.channel.poll_recv(&mut cx), Poll::Ready(Ok(1)));
//     assert_eq!(receiver.channel.poll_recv(&mut cx), Poll::Pending);
//   }
//
//   #[test]
//   fn test_receiver_iter() {
//     let (sender, receiver) = unbounded::<u32>();
//     assert!(sender.send(1).is_ok());
//     assert!(sender.send(2).is_ok());
//
//     let mut iter = receiver.try_iter();
//     assert_eq!(iter.next(), Some(1));
//     assert_eq!(iter.next(), Some(2));
//     assert_eq!(iter.next(), None);
//   }
//
//   #[test]
//   fn test_receiver_future() {
//     let (sender, receiver) = unbounded::<u32>();
//     assert!(sender.send(1).is_ok());
//
//     let future = receiver.recv();
//     assert_eq!(crate::runtime::Runtime::builder().block_on(future), Ok(1));
//   }
//
//   #[test]
//   fn test_stream() {
//     let (sender, receiver) = unbounded::<u32>();
//     let mut stream = Box::pin(receiver);
//     let waker = noop_waker();
//     let mut cx = Context::from_waker(&waker);
//
//     assert!(sender.send(1).is_ok());
//     assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(Some(1)));
//     assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Pending);
//   }
//
//   #[test]
//   fn test_drop_behavior() {
//     let (sender, receiver) = unbounded::<u32>();
//     drop(receiver);
//     assert_eq!(sender.send(1), Err(ReceiverDroppedError));
//
//     let (sender, receiver) = unbounded::<u32>();
//     drop(sender);
//     assert_eq!(receiver.try_recv(), Err(RecvError::Disconnected));
//   }
//
//   fn noop_waker() -> Waker {
//     use std::task::Wake;
//     struct NoopWaker;
//     impl Wake for NoopWaker {
//       fn wake(self: StdArc<Self>) {}
//     }
//     let raw_waker = StdArc::new(NoopWaker).into();
//     unsafe { Waker::from_raw(raw_waker) }
//   }
// }

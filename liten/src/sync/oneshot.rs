mod not_sync;
pub use not_sync::{Receiver, Sender};
pub mod sync;

use std::sync::Arc;

/// A oneshot channel is a channel in which a value can only be sent once, and when sent the
/// sender is dropped. Simirlarly, The receiver can only receive data once, and is then dropped.
///
///
/// If a channel is guarranteed to send one piece of data, a number of optimisations can be made.
/// This makes oneshot channels very optimised for a async runtime.
pub fn channel<V>() -> (not_sync::Sender<V>, not_sync::Receiver<V>) {
  let channel = Arc::new(not_sync::Channel::new());

  (
    not_sync::Sender::new(channel.clone()),
    not_sync::Receiver::new(channel.clone()),
  )
}

pub fn sync_channel<V>() -> (sync::Sender<V>, sync::Receiver<V>) {
  let channel = Arc::new(sync::Inner::new());

  (sync::Sender::new(channel.clone()), sync::Receiver::new(channel.clone()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::runtime::Runtime;
  use std::time::Duration;
  use tracing::Level;

  #[test]
  // https://github.com/liten-rs/liten/issues/13
  #[ignore]
  fn simple() {
    let sub = tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(Level::TRACE)
      .finish();

    let _ = tracing::subscriber::set_global_default(sub);

    Runtime::new().block_on(async {
      use crate::task;

      let (sender, receiver) = channel();

      let handle = task::spawn(async move {
        sender.send(2).unwrap();
      });

      task::spawn(async move {
        assert!(receiver.await.unwrap() == 2);
      })
      .await
      .unwrap();

      handle.await.unwrap();
    })
  }

  #[test]
  // https://github.com/liten-rs/liten/issues/13
  #[ignore]
  fn sync_error_on_drop() {
    let sub = tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(Level::TRACE)
      .finish();

    let _ = tracing::subscriber::set_global_default(sub);

    Runtime::new().block_on(async {
      let (sender, receiver) = sync_channel::<u8>();
      drop(sender);
      assert!(receiver.await == Err(sync::OneshotError::ChannelDropped));

      let (sender, receiver) = sync_channel::<u8>();
      drop(receiver);
      assert!(sender.send(0).await == Err(sync::OneshotError::ChannelDropped));
    });
  }

  #[test]
  #[ignore]
  #[tracing::instrument]
  fn sync_simultaniously() {
    let sub = tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(Level::TRACE)
      .without_time()
      .finish();

    let _ = tracing::subscriber::set_global_default(sub);
    Runtime::new().block_on(async move {
      let (sender, receiver) = sync_channel::<u8>();
      let handler1 = crate::task::spawn(async {
        sender.send(0).await.unwrap();
        std::thread::sleep(Duration::from_millis(400));
      });

      let handler2 = crate::task::spawn(async {
        let result = receiver.await;
        assert_eq!(result, Ok(0));
      });

      handler2.await.unwrap();
      handler1.await.unwrap();
    });
  }
}

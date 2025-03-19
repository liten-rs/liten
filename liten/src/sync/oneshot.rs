mod not_sync;
pub use not_sync::{Receiver, Sender};
use tracing::Level;
mod sync;

use std::{sync::Arc, time::Duration};

use crate::runtime::Runtime;

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

pub fn sync_channel<V>() -> (sync::SyncSender<V>, sync::SyncReceiver<V>) {
  let channel = Arc::new(sync::SyncChannel::new());

  (
    sync::SyncSender::new(channel.clone()),
    sync::SyncReceiver::new(channel.clone()),
  )
}

#[test]
// https://github.com/liten-rs/liten/issues/13
#[ignore]
async fn simple() {
  let sub = tracing_subscriber::FmtSubscriber::builder()
    .with_max_level(Level::TRACE)
    .finish();

  tracing::subscriber::set_global_default(sub);

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

  tracing::subscriber::set_global_default(sub);

  Runtime::new().block_on(async {
    let (sender, receiver) = sync_channel::<u8>();
    drop(sender);
    assert!(receiver.await == Err(sync::SyncReceiverError::SenderDroppedError));

    let (sender, receiver) = sync_channel::<u8>();
    drop(receiver);
    assert!(
      sender.send(0).await == Err(sync::SyncSenderError::ReceiverDroppedError)
    );
  });
}

#[test]
#[tracing::instrument]
fn sync_simultaniously() {
  let sub = tracing_subscriber::FmtSubscriber::builder()
    .with_max_level(Level::TRACE)
    .finish();

  tracing::subscriber::set_global_default(sub);
  Runtime::new().block_on(async move {
    let (sender, receiver) = sync_channel::<u8>();
    let handler1 = crate::task::spawn(async {
      sender.send(0).await.unwrap();
    });

    let handler2 = crate::task::spawn(async {
      let result = receiver.await;
      assert!(result == Ok(0));
    });

    handler2.await.unwrap();
    handler1.await.unwrap();
  });
}

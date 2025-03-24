pub use not_sync::{Receiver, Sender};
pub mod not_sync;
pub mod sync;

use crate::loom::sync::Arc;

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

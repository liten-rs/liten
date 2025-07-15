//! A once-only send value channel.

use std::ptr::NonNull;

pub use imp::*;
mod imp;

/// A oneshot channel is a channel in which a value can only be sent once, and when sent the
/// sender is dropped. Simirlarly, The receiver can only receive data once, and is then dropped.
///
///
/// If a channel is guarranteed to send one piece of data, a number of optimisations can be made.
/// This makes oneshot channels very optimised for a async runtime.
pub fn channel<V>() -> (imp::Sender<V>, imp::Receiver<V>) {
  let channel =
    NonNull::new(Box::into_raw(Box::new(imp::Inner::new()))).unwrap();

  (imp::Sender::new(channel), imp::Receiver::new(channel))
}

mod imp;
use std::sync::Arc;

pub use imp::*;
pub fn pulse() -> (imp::PulseSender, imp::PulseReceiver) {
  let inner = Arc::new(imp::State::default());
  (imp::PulseSender::new(inner.clone()), imp::PulseReceiver::new(inner))
}

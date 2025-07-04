mod imp;
pub fn pulse() -> (imp::PulseSender, imp::PulseReceiver) {
  let inner = imp::State::default();
  (imp::PulseSender::new(inner.clone()), imp::PulseReceiver::new(inner))
}

lint:
  RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps
  cargo clippy --all-features
doc:
  RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps

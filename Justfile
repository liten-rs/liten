loom-test:
  RUSTFLAGS="--cfg loom -C debug_assertions" cargo nextest r --release --features _loom_testing --lib --fail-fast



test:
  cargo nextest r --all-features

test-doc:
  cargo test --doc --all-features

miri-test:
  # miriflgas is for issue with time syscalls
  MIRIFLAGS="-Zmiri-permissive-provenance -Zmiri-disable-isolation" cargo miri nextest r --target x86_64-unknown-linux-gnu --all-features

miri-test-watch:
  # miriflgas is for issue with time syscalls
  MIRIFLAGS="-Zmiri-permissive-provenance -Zmiri-disable-isolation" cargo watch -x "miri nextest r --target x86_64-unknown-linux-gnu --all-features"

lint:
  RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps
  cargo clippy --all-features
doc:
  RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps

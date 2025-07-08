loom-test:
  RUSTFLAGS="--cfg liten=\"loom\"" cargo nextest r --release --all-features

test:
  cargo nextest r --all-features --nocapture

test-doc:
  cargo test --doc --all-features

miri-test:
  # miriflgas is for issue with crossbeam-deque
  MIRIFLAGS="-Zmiri-permissive-provenance -Zmiri-disable-isolation" cargo miri nextest r --target x86_64-unknown-linux-gnu --all-features

miri-test-watch:
  # miriflgas is for issue with crossbeam-deque
  MIRIFLAGS="-Zmiri-permissive-provenance -Zmiri-disable-isolation" cargo watch -x "miri nextest r --target x86_64-unknown-linux-gnu --all-features"

check:
  cargo check --all-features

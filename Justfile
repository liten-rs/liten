watch-test:
  RUSTFLAGS="--cfg loom" cargo watch -x "test --release"

test:
  RUSTFLAGS="--cfg loom" cargo test --release

miri-test:
  # miriflgas is for issue with crossbeam-deque
  MIRIFLAGS=-Zmiri-permissive-provenance cargo miri test --target x86_64-unknown-linux-gnu

watch-test:
  RUSTFLAGS="--cfg loom" cargo watch -x "test --release"

test:
  RUSTFLAGS="--cfg loom" cargo nextest r --release

miri-test:
  # miriflgas is for issue with crossbeam-deque
  MIRIFLAGS=-Zmiri-permissive-provenance cargo miri nextest r --target x86_64-unknown-linux-gnu

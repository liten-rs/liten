watch-test:
  RUSTFLAGS="--cfg loom" cargo watch -x "test --release"

test:
  RUSTFLAGS="--cfg loom" cargo test --release

miri-test:
  cargo miri test

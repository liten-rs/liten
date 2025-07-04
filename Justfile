loom-test:
  RUSTFLAGS="--cfg liten=\"loom\"" cargo nextest r --release

test:
  cargo nextest r
  cargo test --doc

miri-test:
  # miriflgas is for issue with crossbeam-deque
  MIRIFLAGS="-Zmiri-permissive-provenance -Zmiri-disable-isolation" cargo miri nextest r --target x86_64-unknown-linux-gnu

check:
  cargo check --all-features
#![cfg(loom)]

use liten::runtime::Runtime;

#[test]
fn builder() {
  loom::model(|| {
    Runtime::builder().num_workers(1);
  })
}

#[test]
fn only_builder() {
  loom::model(|| {
    Runtime::builder().num_workers(1).block_on(async {});
  })
}

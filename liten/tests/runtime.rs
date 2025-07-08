use liten::runtime::Runtime;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[liten::internal_test]
fn builder_and_block_on_integration() {
  let counter = Arc::new(AtomicUsize::new(0));
  let c = counter.clone();
  let res = Runtime::single_threaded().block_on(async move {
    c.fetch_add(1, Ordering::SeqCst);
    42
  });
  assert_eq!(res, 42);
  assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// #[liten::internal_test]
// #[should_panic]
// fn nested_runtime_panics_integration() {
//     Runtime::builder().block_on(async {
//         Runtime::builder().block_on(async {});
//     });
// }

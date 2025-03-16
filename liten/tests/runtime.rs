#[test]
fn task_starts() {
  liten::runtime::Runtime::new().block_on(async {
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let handle = liten::task::spawn({
      let counter = counter.clone();
      async move {
        counter.clone().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      }
    });

    let handle2 = liten::task::spawn({
      let counter = counter.clone();
      async move {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      }
    });

    let _ = handle.await;
    let _ = handle2.await;

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
  })
}

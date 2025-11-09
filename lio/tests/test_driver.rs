use lio::loom::test_utils::model;

#[test]
fn test_driver() {
  model(|| {
    lio::init();
    lio::shutdown();
  });
}

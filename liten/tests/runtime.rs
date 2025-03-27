use liten::runtime::Runtime;

#[liten::internal_test]
fn builder() {
  Runtime::builder().num_workers(1);
}

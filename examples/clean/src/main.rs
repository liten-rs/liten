#[liten::main]
async fn main() {
  let _ = liten::task::spawn(async {}).await;
}

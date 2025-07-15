use liten::runtime::Runtime;

fn main() {
  Runtime::single_threaded().block_on(async {})
}

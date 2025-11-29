use std::time::Duration;

fn main() {
  // lio::init();

  lio::write(2, vec![39], 0).when_done(|k| {
    dbg!(k);
  });
  println!("nice");
  // lio::socket(socket2::Domain::IPV4, socket2::Type::STREAM, None);

  std::thread::sleep(Duration::from_secs(1));
}

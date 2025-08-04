use liten::runtime::Runtime;

fn main() {
  Runtime::single_threaded().block_on(async {
    let tesing = liten::io::net::socket::Socket::new(
      socket2::Domain::IPV4,
      socket2::Type::STREAM,
    )
    .await
    .unwrap();
    // let testing = liten::io::fs::read("./README.md").await.unwrap();
    // println!("{:?}", testing);
  })
}

use std::{
  sync::atomic::{AtomicU8, Ordering},
  time::Duration,
};

use liten::{
  actor::{Actor, ActorResult},
  // blocking::unblock,
  // runtime::Runtime,
  time::sleep,
};
// use tracing_subscriber::fmt;
//
// static COUNT: AtomicU8 = AtomicU8::new(0);
//
// struct DemoActor;
//
// impl Actor<u8> for DemoActor {
//   type Output = u8;
//   async fn handle(self: &mut Self, input: &u8) -> ActorResult<Self::Output> {
//     ActorResult::Result(COUNT.fetch_add(*input, Ordering::AcqRel) + *input)
//   }
// }

#[liten::main]
async fn main() {
  sleep(Duration::from_secs(10)).await;
  // tracing::subscriber::set_global_default(
  //   fmt().with_max_level(tracing::Level::TRACE).finish(),
  // )
  // .unwrap();
  // Runtime::builder().num_workers(1).block_on(async {
  //   let handle = DemoActor.start();
  //   println!("wha");
  //
  //   handle.send(1).await;
  //   // println!("wha");
  //   handle.send(1).await;
  //   handle.send(1).await;
  //   //
  //   handle.stop().await;
  //
  //   assert_eq!(unblock(|| 5).await, 5);
  // })
}

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  liten::runtime::Runtime::builder()
    .disable_work_stealing()
    .num_workers(4)
    .block_on(async {
      let handle = liten::task::spawn(async {
        tracing::info!("Very nice");
        "yes"
      });
      println!("program stop -----");

      let reslut = handle.await;

      println!("{:#?}", reslut);

      Ok(())
    })
}

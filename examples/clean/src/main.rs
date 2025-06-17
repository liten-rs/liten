use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  liten::runtime::Runtime::builder().block_on(async {
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

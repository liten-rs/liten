use std::error::Error;

use liten::io::fs::read_to_string;

#[liten::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let result = read_to_string("./README.md").await?;
  println!("{result}");
  Ok(())
}

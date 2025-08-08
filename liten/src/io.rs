mod driver;
mod utils;

pub(crate) use driver::Driver;
pub mod fs;
pub mod net;

pub type BufResult<T, E, B> = (Result<T, E>, B);

use std::io;

mod accept;
mod bind;
mod close;
mod connect;
mod listen;
mod read;
mod recv;
mod send;
mod socket;
mod tee;
mod truncate;
mod write;

pub use accept::*;
pub use bind::*;
pub use close::*;
pub use connect::*;
pub use listen::*;
pub use read::*;
pub use recv::*;
pub use send::*;
pub use socket::*;
pub use tee::*;
pub use truncate::*;
pub use write::*;

/// Done to disallow someone creating a operation outside of lio.
trait Sealed {}
impl<O: Operation> Sealed for O {}

// Things that implement this trait represent a command that can be executed using io-uring.
// TODO: Maybe combine output and result?
#[allow(private_bounds)]
pub trait Operation: Sealed {
  type Output: Sized;
  type Result; // = io::Result<Self::Output>;
  fn create_entry(&self) -> io_uring::squeue::Entry;
  // This is guarranteed after this has completed and only fire ONCE.
  // i32 is guarranteed to be >= 0.
  fn result(&mut self, _ret: io::Result<i32>) -> Self::Result;
}

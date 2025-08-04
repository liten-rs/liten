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
pub use write::*;

// Things that implement this trait represent a command that can be executed using io-uring.
pub trait Operation {
  type Output: Sized;
  fn create_entry(&self) -> io_uring::squeue::Entry;
  // This is guarranteed after this has completed and only fire ONCE.
  fn result(&mut self) -> Self::Output;
}

macro_rules! syscall {
  ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
      #[allow(unused_unsafe)]
      let res = unsafe { libc::$fn($($arg, )*) };
      if res == -1 {
          Err(std::io::Error::last_os_error())
      } else {
          Ok(res)
      }
  }};
}

use std::io;

#[cfg(not(linux))]
use std::os::fd::RawFd;

mod accept;
mod bind;
mod close;
mod connect;
mod listen;
mod net_utils;
mod openat;
mod read;
mod recv;
mod send;
mod socket;

mod fsync;
mod linkat;
mod shutdown;
mod symlink;
#[cfg(linux)]
mod tee;
#[cfg(linux)]
mod timeout;
mod truncate;
mod write;

pub use accept::*;
pub use bind::*;
pub use close::*;
pub use connect::*;
pub use fsync::*;
pub use linkat::*;
pub use listen::*;
pub use openat::*;
pub use read::*;
pub use recv::*;
pub use send::*;
pub use shutdown::*;
pub use socket::*;
pub use symlink::*;
#[cfg(linux)]
pub use timeout::*;

#[cfg(linux)]
pub use tee::*;

pub use truncate::*;
pub use write::*;

/// Done to disallow someone creating a operation outside of lio, which will cause issues.
trait Sealed {}
impl<O: Operation> Sealed for O {}

// Things that implement this trait represent a command that can be executed using io-uring.
#[allow(private_bounds)]
pub trait Operation: Sealed {
  type Output: Sized;
  type Result; // = most often io::Result<Self::Output>;

  #[cfg(linux)]
  const OPCODE: u8;

  #[cfg(linux)]
  fn entry_supported(probe: &io_uring::Probe) -> bool {
    probe.is_supported(Self::OPCODE)
  }

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry;

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType>;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd>;

  fn run_blocking(&self) -> io::Result<i32>;
  /// This is guarranteed to fire after this has completed and only fire ONCE.
  /// i32 is guarranteed to be >= 0.
  fn result(&mut self, _ret: io::Result<i32>) -> Self::Result;
}

pub enum EventType {
  Read,
  Write,
}

macro_rules! os_linux {
   ($($item:item)*) => {
       $(
            #[cfg(target_os = "linux")]
            $item
        )*
    }
}
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

mod accept;
mod bind;
mod close;
mod connect;
mod listen;
mod openat;
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
pub use openat::*;
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

  os_linux! {
    const OPCODE: u8;

    fn supported() -> bool {
      io_uring::Probe::new().is_supported(Self::OPCODE)
    }

    fn create_entry(&self) -> io_uring::squeue::Entry;
    fn run_blocking(&self) -> io::Result<i32>;
    // This is guarranteed after this has completed and only fire ONCE.
    // i32 is guarranteed to be >= 0.
    fn result(&mut self, _ret: io::Result<i32>) -> Self::Result;
  }
}

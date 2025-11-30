use crate::op::DetachSafe;
#[cfg(not(linux))]
use std::io::ErrorKind;
use std::time::Duration;

#[cfg(linux)]
use io_uring::{opcode, squeue, types::Timespec};

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Timeout {
  timespec: Timespec,
}

// wtf are you doing waiting and then not waiting??
unsafe impl DetachSafe for Timeout {}

impl Timeout {
  pub(crate) fn new(duration: Duration) -> Self {
    Self {
      timespec: Timespec::new()
        .sec(duration.as_secs())
        .nsec(duration.subsec_nanos()),
    }
  }
}

impl Operation for Timeout {
  type Output = ();
  type Result = std::io::Result<Self::Output>;
  fn result(&mut self, res: std::io::Result<i32>) -> Self::Result {
    match res {
      Ok(v) => {
        assert!(v == 0);
        Ok(())
      }
      Err(err) => {
        if let Some(inner_err) = err.raw_os_error() {
          match inner_err {
            libc::ETIME => Ok(()),
            _ => Err(err),
          }
        } else {
          Err(err)
        }
      }
    }
  }

  #[cfg(linux)]
  const OPCODE: u8 = 11;

  #[cfg(linux)]
  fn create_entry(&mut self) -> squeue::Entry {
    opcode::Timeout::new(&self.timespec as *const _).build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(not(linux))]
  fn run_blocking(&self) -> std::io::Result<i32> {
    todo!();
    // syscall!(timeout(self.fd, self.size as i64))
  }
}

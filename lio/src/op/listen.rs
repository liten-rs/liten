use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::types::Fd;

use super::Operation;

pub struct Listen {
  fd: RawFd,
  backlog: i32,
}

impl Listen {
  pub fn new(fd: RawFd) -> Self {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "horizon")] {
            // The 3DS doesn't support a big connection backlog. Sometimes
            // it allows up to about 37, but other times it doesn't even
            // accept 32. There may be a global limitation causing this.
            let backlog = 20;
        } else if #[cfg(target_os = "haiku")] {
            // Haiku does not support a queue length > 32
            // https://github.com/haiku/haiku/blob/979a0bc487864675517fb2fab28f87dc8bf43041/headers/posix/sys/socket.h#L81
            let backlog = 32;
        } else {
            // The default for all other platforms
            let backlog = 128;
        }
    }
    Self { fd, backlog }
  }
}

impl Operation for Listen {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 57;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Listen::new(Fd(self.fd), self.backlog).build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(listen(self.fd, self.backlog))
  }
}

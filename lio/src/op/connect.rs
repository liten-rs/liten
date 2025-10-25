use std::os::fd::RawFd;

#[cfg(linux)]
use io_uring::types::Fd;
use socket2::SockAddr;

use super::Operation;

pub struct Connect {
  fd: RawFd,
  addr: SockAddr,
}

impl Connect {
  pub fn new(fd: RawFd, addr: SockAddr) -> Self {
    Self { fd, addr }
  }
}

impl Operation for Connect {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 16;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Connect::new(
      Fd(self.fd),
      self.addr.as_ptr().cast::<libc::sockaddr>(),
      self.addr.len(),
    )
    .build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    // Check SO_ERROR
    // let mut so_error: i32 = 0;
    // let mut len = mem::size_of::<i32>() as libc::socklen_t;
    // let test = syscall!(getsockopt(
    //   self.fd,
    //   libc::SOL_SOCKET,
    //   libc::SO_ERROR,
    //   &mut so_error as *mut _ as *mut libc::c_void,
    //   &mut len,
    // ));

    let result = syscall!(connect(
      self.fd,
      self.addr.as_ptr().cast::<libc::sockaddr>(),
      self.addr.len(),
    ));

    // Macos doesn't silently fail if socket is already connected. So we just ignore it if that
    // would happen.
    #[cfg(macos)]
    if let Err(ref err) = result {
      if let Some(errno) = err.raw_os_error() {
        if errno == 56 {
          return Ok(0); // 0 is dummy variable. prob gonna change trait impl.
        }
      }
    };

    result
  }
}

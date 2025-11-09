use std::io;

use std::os::fd::RawFd;

use crate::op::EventType;

use super::Operation;

pub struct Socket {
  domain: socket2::Domain,
  ty: socket2::Type,
  proto: Option<socket2::Protocol>,
}

impl Socket {
  pub fn new(
    domain: socket2::Domain,
    ty: socket2::Type,
    proto: Option<socket2::Protocol>,
  ) -> Self {
    Self { domain, ty, proto }
  }

  fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let mut nonblocking = true as libc::c_int;
    syscall!(ioctl(fd, libc::FIONBIO, &mut nonblocking)).map(drop)
  }

  fn disable_sigpipe(fd: RawFd) -> io::Result<()> {
    type T = i32;
    let opt: T = 1;

    // DragonFlyBSD, FreeBSD and NetBSD use `SO_NOSIGPIPE` as a `setsockopt`
    // flag to disable `SIGPIPE` emission on socket.
    #[cfg(any(
      target_os = "freebsd",
      target_os = "netbsd",
      target_os = "dragonfly"
    ))]
    syscall!(setsockopt(
      fd,
      libc::SOL_SOCKET,
      libc::SO_NOSIGPIPE,
      &opt as *const T as *const libc::c_void,
      std::mem::size_of::<T>() as u32
    ))?;

    // macOS and iOS use `SO_NOSIGPIPE` as a `setsockopt`
    // flag to disable `SIGPIPE` emission on socket.
    #[cfg(target_vendor = "apple")]
    syscall!(setsockopt(
      fd,
      libc::SOL_SOCKET,
      libc::SO_NOSIGPIPE,
      &opt as *const T as *const libc::c_void,
      std::mem::size_of::<T>() as u32
    ))?;

    Ok(())
  }

  pub fn set_reuseaddr(fd: RawFd) -> io::Result<()> {
    type T = i32;
    let opt: T = 1;

    syscall!(setsockopt(
      fd,
      libc::SOL_SOCKET,
      libc::SO_REUSEADDR,
      &opt as *const T as *const libc::c_void,
      std::mem::size_of::<T>() as u32
    ))
    .map(drop)
  }

  #[cfg(not(any(
    target_env = "newlib",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "emscripten",
    target_os = "fuchsia",
    target_os = "l4re",
    target_os = "linux",
    target_os = "cygwin",
    target_os = "haiku",
    target_os = "redox",
    target_os = "vxworks",
    target_os = "nto",
  )))]
  pub fn set_cloexec(fd: RawFd) -> io::Result<()> {
    syscall!(ioctl(fd, libc::FIOCLEX))?;
    Ok(())
  }
}

impl Operation for Socket {
  #[cfg(unix)]
  type Output = std::os::fd::RawFd;

  #[cfg(unix)]
  type Result = std::io::Result<Self::Output>;

  #[doc = r" File descriptor returned from the operation."]
  fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
    fd
  }

  #[cfg(linux)]
  const OPCODE: u8 = 45;

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::Socket::new(
      self.domain.into(),
      self.ty.into(),
      self.proto.unwrap_or(0.into()).into(),
    )
    .build()
  }

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  fn run_blocking(&self) -> io::Result<i32> {
    #[cfg(any(
      target_os = "android",
      target_os = "dragonfly",
      target_os = "freebsd",
      target_os = "illumos",
      target_os = "hurd",
      target_os = "linux",
      target_os = "netbsd",
      target_os = "openbsd",
      target_os = "cygwin",
      target_os = "nto",
      target_os = "solaris",
    ))]
    let fd = {
      let fd = syscall!(socket(
        self.domain.into(),
        libc::c_int::from(self.ty) | libc::SOCK_CLOEXEC,
        self.proto.unwrap_or(0.into()).into()
      ))?;

      fd
    };

    #[cfg(not(any(
      target_os = "android",
      target_os = "dragonfly",
      target_os = "freebsd",
      target_os = "illumos",
      target_os = "hurd",
      target_os = "linux",
      target_os = "netbsd",
      target_os = "openbsd",
      target_os = "cygwin",
      target_os = "nto",
      target_os = "solaris",
    )))]
    let fd = {
      let fd = syscall!(socket(
        self.domain.into(),
        self.ty.into(),
        self.proto.unwrap_or(0.into()).into()
      ))?;

      #[cfg(not(any(
        target_env = "newlib",
        target_os = "emscripten",
        target_os = "fuchsia",
        target_os = "l4re",
        target_os = "haiku",
        target_os = "redox",
        target_os = "vxworks",
      )))]
      Self::set_cloexec(fd)?;

      fd
    };

    Self::disable_sigpipe(fd)?;

    // Remove blocking by kernel if no result is available. Linux uses io-uring so it doesn't
    // count.
    #[cfg(not(linux))]
    Self::set_nonblocking(fd)?;

    Self::set_reuseaddr(fd)?;

    Ok(fd)
  }
}

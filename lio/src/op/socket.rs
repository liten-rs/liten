use std::io;
use std::os::fd::RawFd;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct Socket {
  domain: socket2::Domain,
  ty: socket2::Type,
  proto: Option<socket2::Protocol>,
}

impl Socket {
  pub(crate) fn new(
    domain: socket2::Domain,
    ty: socket2::Type,
    proto: Option<socket2::Protocol>,
  ) -> Self {
    Self { domain, ty, proto }
  }

  /// Sets the socket to non-blocking mode.
  ///
  /// **What it does:**
  /// Configures the socket so that I/O operations (read/write/accept/connect) return
  /// immediately with EAGAIN/EWOULDBLOCK instead of blocking the thread when no data
  /// is available or the operation cannot complete immediately.
  ///
  /// **Why it's needed:**
  /// - REQUIRED for async I/O with kqueue/epoll - these mechanisms only notify when
  ///   a socket is ready, but the actual syscall must be non-blocking
  /// - Without this, a single slow connection could block the entire async runtime
  /// - Enables multiplexing thousands of connections on a single thread
  ///
  /// **Platform differences:**
  /// - Linux: Can use SOCK_NONBLOCK flag atomically during socket creation
  /// - macOS/others: Must set after creation via ioctl, creating a small race window
  #[cfg(not(linux))]
  fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let mut nonblocking = true as libc::c_int;
    syscall!(ioctl(fd, libc::FIONBIO, &mut nonblocking)).map(drop)
  }

  /// Disables SIGPIPE signal emission when writing to a closed socket.
  ///
  /// **What it does:**
  /// Prevents the kernel from sending SIGPIPE signal to the process when attempting
  /// to write to a socket that has been closed by the peer. Instead, the write call
  /// returns EPIPE error.
  ///
  /// **Why it's needed:**
  /// - Default SIGPIPE behavior terminates the process (unless explicitly handled)
  /// - In a server handling many connections, any client disconnect would crash the server
  /// - Allows graceful error handling instead of process termination
  ///
  /// **Platform differences:**
  /// - BSD/macOS: Use SO_NOSIGPIPE socket option (set per-socket)
  /// - Linux: Use MSG_NOSIGNAL flag on each send() call (not set here)
  /// - This is a no-op on platforms without SO_NOSIGPIPE
  fn disable_sigpipe(#[allow(unused)] fd: RawFd) -> io::Result<()> {
    #[cfg(any(
      target_os = "freebsd",
      target_os = "netbsd",
      target_os = "dragonfly",
      target_vendor = "apple"
    ))]
    {
      let opt: i32 = 1;
      syscall!(setsockopt(
        fd,
        libc::SOL_SOCKET,
        libc::SO_NOSIGPIPE,
        &opt as *const i32 as *const libc::c_void,
        std::mem::size_of::<i32>() as libc::socklen_t
      ))
      .map(drop)?;
    }

    Ok(())
  }

  /// Enables SO_REUSEADDR socket option.
  ///
  /// **What it does:**
  /// Allows the socket to bind to an address that is in TIME_WAIT state from a
  /// previous connection. Also allows multiple sockets to bind to the same address
  /// if they use different ports (or same port in some cases).
  ///
  /// **Why it's needed:**
  /// - When a server restarts, the previous socket may be in TIME_WAIT (lasts 2-4 minutes)
  /// - Without this, bind() fails with EADDRINUSE until TIME_WAIT expires
  /// - Essential for development (rapid restart cycles) and production (quick failover)
  ///
  /// **Platform differences:**
  /// - Works consistently across all Unix platforms
  /// - Slightly different semantics on different OSes, but generally safe for servers
  ///
  /// **Consequences if omitted:**
  /// - Cannot restart server for 2-4 minutes after shutdown (TIME_WAIT period)
  /// - bind() returns EADDRINUSE error
  /// - Forces use of different port or waiting for TIME_WAIT to expire
  ///
  /// **Note:** This uses `libc::socklen_t` for the length parameter, which is
  /// platform-dependent (not always u32). Using u32 could cause failures on some
  /// 64-bit platforms where socklen_t is defined differently.
  fn set_reuseaddr(fd: RawFd) -> io::Result<()> {
    let opt: i32 = 1;
    syscall!(setsockopt(
      fd,
      libc::SOL_SOCKET,
      libc::SO_REUSEADDR,
      &opt as *const i32 as *const libc::c_void,
      std::mem::size_of::<i32>() as libc::socklen_t
    ))
    .map(drop)
  }

  /// Sets the close-on-exec flag on the socket file descriptor.
  ///
  /// **What it does:**
  /// Sets the FD_CLOEXEC flag, which causes this file descriptor to be automatically
  /// closed when the process executes a new program via exec() family of functions.
  ///
  /// **Why it's needed:**
  /// - Security: Prevents child processes from inheriting sensitive file descriptors
  /// - Resource management: Child processes shouldn't keep parent's sockets open
  /// - Prevents port binding conflicts if child process continues running
  ///
  /// **Platform differences:**
  /// - Linux/BSD: Can use SOCK_CLOEXEC flag atomically during socket creation
  /// - macOS: Must use FIOCLEX ioctl after creation (small race window)
  /// - Some embedded systems: Not available (feature-gated out)
  ///
  /// **Consequences if omitted:**
  /// - Child processes inherit the socket FD and can read/write/close it
  /// - Security risk: sensitive connections leak to untrusted child processes
  /// - Resource leak: socket stays open even if parent closes it (child keeps ref)
  /// - Port remains bound even after parent exits if child is still running
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
  fn set_cloexec(fd: RawFd) -> io::Result<()> {
    syscall!(ioctl(fd, libc::FIOCLEX)).map(drop)
  }

  /// Sets up all socket options in sequence.
  ///
  /// **Error handling:**
  /// This function propagates errors using `?` operator. If any setup step fails,
  /// the error is returned immediately. The caller MUST close the socket FD to
  /// prevent resource leaks (see run_blocking implementation).
  fn setup_socket_options(&self, fd: RawFd) -> io::Result<()> {
    // SIGPIPE handling (BSD/macOS only)
    Self::disable_sigpipe(fd)?;

    // Non-blocking mode (required for kqueue/epoll, not needed on Linux with io_uring)
    #[cfg(not(linux))]
    Self::set_nonblocking(fd)?;

    // SO_REUSEADDR (allows quick rebind)
    Self::set_reuseaddr(fd)?;

    Ok(())
  }
}

impl Operation for Socket {
  #[cfg(unix)]
  type Output = std::os::fd::RawFd;

  #[cfg(unix)]
  type Result = std::io::Result<Self::Output>;

  fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
    fd
  }

  #[cfg(linux)]
  const OPCODE: u8 = 45;

  #[cfg(linux)]
  fn create_entry(&mut self) -> io_uring::squeue::Entry {
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

  /// Creates a new socket and configures it with essential options.
  ///
  /// **Critical: Error Cleanup Pattern**
  ///
  /// This implementation demonstrates the essential error handling pattern for socket
  /// creation: if socket() succeeds but any subsequent setup fails, we MUST close the
  /// file descriptor before returning the error.
  ///
  /// **Why error cleanup is critical:**
  /// 1. Socket creation (socket syscall) allocates a file descriptor
  /// 2. If setup_socket_options() fails, we have an FD but return an error
  /// 3. Without cleanup, the FD leaks (never closed, never used)
  /// 4. Each leak consumes one FD from the process limit (typically 1024-65536)
  /// 5. Eventually hits EMFILE (too many open files), breaking the application
  ///
  /// **The pattern:**
  /// ```rust
  /// let fd = socket(...)?;  // FD allocated here
  ///
  /// match setup(fd) {
  ///     Ok(()) => Ok(fd),    // Success: return the FD
  ///     Err(e) => {
  ///         close(fd);        // Failure: MUST clean up before returning error
  ///         Err(e)
  ///     }
  /// }
  /// ```
  ///
  /// **Platform-specific behavior:**
  ///
  /// Linux/BSD with SOCK_CLOEXEC support:
  /// - Sets CLOEXEC atomically during socket creation (no race window)
  /// - Non-blocking set separately on non-Linux platforms
  ///
  /// macOS and platforms without SOCK_CLOEXEC:
  /// - Creates socket first (blocking, no CLOEXEC)
  /// - Sets CLOEXEC via ioctl (small race: child could exec before this)
  /// - Sets non-blocking via ioctl
  /// - If any ioctl fails, closes the socket and returns error (no leak)
  fn run_blocking(&self) -> io::Result<i32> {
    // Path 1: Platforms with SOCK_CLOEXEC support (atomic CLOEXEC flag)
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
      // Create socket with CLOEXEC set atomically (no race window)
      let fd = syscall!(socket(
        self.domain.into(),
        libc::c_int::from(self.ty) | libc::SOCK_CLOEXEC,
        self.proto.unwrap_or(0.into()).into()
      ))?;

      // CRITICAL ERROR CLEANUP: If setup fails, close FD before returning error
      match self.setup_socket_options(fd) {
        Ok(()) => Ok(fd),
        Err(e) => {
          unsafe { libc::close(fd) };
          Err(e)
        }
      }?
    };

    // Path 2: Platforms without SOCK_CLOEXEC (macOS, etc.)
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
      // Create socket without CLOEXEC (will set separately)
      let fd = syscall!(socket(
        self.domain.into(),
        self.ty.into(),
        self.proto.unwrap_or(0.into()).into()
      ))?;

      // Set CLOEXEC if supported on this platform
      #[cfg(not(any(
        target_env = "newlib",
        target_os = "emscripten",
        target_os = "fuchsia",
        target_os = "l4re",
        target_os = "haiku",
        target_os = "redox",
        target_os = "vxworks",
      )))]
      {
        // CRITICAL ERROR CLEANUP: If CLOEXEC fails, close FD before returning error
        if let Err(e) = Self::set_cloexec(fd) {
          unsafe { libc::close(fd) };
          return Err(e);
        }
      }

      // CRITICAL ERROR CLEANUP: If setup fails, close FD before returning error
      match self.setup_socket_options(fd) {
        Ok(()) => Ok(fd),
        Err(e) => {
          unsafe { libc::close(fd) };
          Err(e)
        }
      }?
    };

    Ok(fd)
  }
}

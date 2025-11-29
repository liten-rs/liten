//! # Lio - Platform-Independent Async I/O Library
//!
//! Lio is a high-performance, platform-independent async I/O library that uses
//! the most efficient IO for each platform.
//!
//! ## Features
//! - **Zero-copy operations** where possible.
//! - **Automatic fallback** to blocking operations when async isn't supported.
//! - **Manual control** with high level async API.
//!
//! *Note:* This is a quite low-level library. This library creates resources
//! (fd's) which it doesn't cleanup itself.
//!
//! ## Platform support
//!
//! | Platform   | I/O Mechanism            | Status                  |
//! |------------|--------------------------|-------------------------|
//! | Linux      | io_uring                 | Yes                     |
//! | Windows    | IOCP                     | Not supported (planned) |
//! | macOS      | kqueue                   | Yes                     |
//! | Other Unix | poll/epoll/event ports   | Yes                     |
//!
//!
//! ## Quick Start
//!
//! ```rust
//! use std::os::fd::RawFd;
//!
//! async fn example() -> std::io::Result<()> {
//!     let fd: RawFd = 1; // stdout
//!     let data = b"Hello, World!\n".to_vec();
//!
//!     // Async write operation
//!     let (result, _buf) = lio::write(fd, data, 0).await;
//!     println!("Wrote {} bytes", result?);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Callback-Based I/O
//!
//! Lio supports both Future-based (`.await`) and callback-based execution models.
//! Use callbacks for fire-and-forget operations or when you need results in a
//! different context than the async caller.
//!
//! ```rust
//! use std::sync::mpsc::channel;
//!
//! async fn callback_example() -> std::io::Result<()> {
//!     # let fd = 0;
//!     let buffer = vec![0u8; 1024];
//!     let (tx, rx) = channel();
//!
//!     // Register callback - operation continues in background
//!     lio::read(fd, buffer, 0).when_done(move |(result, buf)| {
//!         match result {
//!             Ok(bytes_read) => {
//!                 println!("Read {} bytes", bytes_read);
//!                 tx.send(buf).unwrap();
//!             }
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     });
//!
//!     // Continue with other work while I/O happens
//!     // ...
//!
//!     // Later, get the result
//!     let buffer = rx.recv().unwrap();
//!     Ok(())
//! }
//! ```
//!
//! **Note**: Once `.when_done()` is called, the operation cannot be polled as a
//! Future. Choose one execution model per operation.
//!
//! ## Examples
//!
//! ### File I/O
//!
//! ```rust
//! use std::ffi::CString;
//!
//! async fn file_operations() -> std::io::Result<()> {
//!     let path = CString::new("/tmp/test.txt").unwrap();
//!     let fd = lio::openat(libc::AT_FDCWD, path, libc::O_CREAT | libc::O_WRONLY).await?;
//!     
//!     let data = b"Hello, async I/O!".to_vec();
//!     let (result_bytes_written, _buf) = lio::write(fd, data, 1 /* stdout fd */).await;
//!
//!     let _ = result_bytes_written?;
//!     
//!     lio::close(fd).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Network I/O
//!
//! ```rust
//! use socket2::{Domain, Type, Protocol};
//!
//! async fn network_operations() -> std::io::Result<()> {
//!     let sock = lio::socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).await?;
//!     let addr = "127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap();
//!     
//!     lio::bind(sock, addr).await?;
//!     lio::listen(sock, 128).await?;
//!     
//!     // Accept connections...
//!
//!     lio::close(sock).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Safety and Threading
//!
//! - The library handles thread management for background I/O processing
//! - Operations can be safely used across different threads
//!
//! ## Error Handling
//!
//! All operations return `std::io::Result<T>` or `BufResult<T, B>` for operations
//! that return buffers. Errors are automatically converted from platform-specific
//! error codes to Rust's standard I/O error types.

use std::{
  ffi::{CString, NulError},
  net::SocketAddr,
  os::fd::RawFd,
};

/// Result type for operations that return both a result and a buffer.
///
/// This is commonly used for read/write operations where the buffer
/// is returned along with the operation result.
pub type BufResult<T, B> = (std::io::Result<T>, B);

#[macro_use]
mod macros;

mod driver;

pub mod op;
use op::*;

mod op_progress;
mod op_registration;

pub use op_progress::OperationProgress;

use crate::driver::Driver;
use std::path::Path;

macro_rules! impl_op {
  // Internal helper: Generate function with common documentation
  (@impl_fn
    { $desc:expr, $operation:ident, $name:ident, [$($arg:ident : $arg_ty:ty),*], $ret:ty }
    $( #[$($doc:tt)*] )*
    { $($body:tt)* }
  ) => {
    #[doc = $desc]
    #[doc = "# Returns"]
    #[doc = concat!("This function returns `OperationProgress<", stringify!($operation), ">`.")]
    #[doc = "This function signature is equivalent to:"]
    #[doc = concat!("```ignore\nasync fn ",stringify!($name), "(", stringify!($($arg),*), ") -> ", stringify!($ret), "\n```")]
    #[doc = "# Behavior"]
    #[doc = "As soon as this function is called, the operation is submitted into the io-driver used by the current platform (for example io-uring). If the user then chooses to drop [`OperationProgress`] before the [`Future`] is ready, the operation will **NOT** tried be cancelled, but instead \"detached\"."]
    #[doc = "\n\nSee more [what methods are available to the return type](crate::OperationProgress#impl-OperationProgress<T>)."]
    $( #[$($doc)*] )*
    $($body)*
  };

  // !detach variant with error type
  (
    !detach $desc:tt,
    $(#[$($doc:tt)*])*
    $operation:ident, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> $ret:ty ; $err:ty
  ) => {
      impl_op!(
          concat!($desc, "\n\n**Not detach safe**\n: This method can be dangerous to call when_done on. It is not `detach safe' which means that resources will not be cleaned up if not handled carefully."),
          $(#[$($doc)*])*
          $operation,
          fn $name($($arg: $arg_ty),*) -> $ret:ty ; $err:ty
      );
  };

  // !detach variant without error type
  (
    !detach $desc:tt,
    $(#[$($doc:tt)*])*
    $operation:ident, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> $ret:ty
  ) => {
      impl_op!(
          concat!($desc, "\n\n**Not detach safe**\n: This method can be dangerous to call when_done on. It is not `detach safe` which means that resources will not be cleaned up if not handled carefully."),
          $(#[$($doc)*])*
          $operation,
          fn $name($($arg: $arg_ty),*) -> $ret
      );
  };

  // With error type - fallible constructor
  (
    $desc:expr,
    $(#[$($doc:tt)*])*
    $operation:ident, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> $ret:ty ; $err:ty
  ) => {
    use op::$operation;

    impl_op!(@impl_fn
      { $desc, $operation, $name, [$($arg : $arg_ty),*], $ret }
      $( #[$($doc)*] )*
      {
        pub fn $name($($arg: $arg_ty),*) -> Result<OperationProgress<$operation>, $err> {
          Ok(Driver::submit($operation::new($($arg),*)?))
        }
      }
    );
  };

  // Without error type - infallible constructor
  (
    $desc:expr,
    $(#[$($doc:tt)*])*
    $operation:ident, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> $ret:ty
  ) => {
    impl_op!(@impl_fn
      { $desc, $operation, $name, [$($arg : $arg_ty),*], $ret }
      $( #[$($doc)*] )*
      {
        pub fn $name($($arg: $arg_ty),*) -> OperationProgress<$operation> {
          Driver::submit($operation::new($($arg),*))
        }
      }
    );
  };

  // Convenience: no description provided
  (
    $(#[$($doc:tt)*])*
    $operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* ) -> $ret:ty
  ) => {
      impl_op!("", $(#[$($doc)*])* $operation, fn $name($($arg: $arg_ty),*) -> $ret);
  };
}

#[cfg(linux)]
use std::time::Duration;

impl_op!(
 "Shuts socket down.",
 /// # Examples
 ///
 /// ```rust
 /// async fn write_example() -> std::io::Result<()> {
 ///     let socket = lio::socket(socket2::Domain::IPV4, socket2::Type::STREAM, None).await?;
 ///     let how = 0;
 ///     lio::shutdown(socket, how).await?;
 ///     Ok(())
 /// }
 /// ```
 Shutdown, fn shutdown(fd: RawFd, how: i32) -> io::Result<()>
);

#[cfg(linux)]
impl_op!(
  "Times out something",
  /// # Examples
  ///
  /// ```rust
  /// use lio::timeout;
  /// use std::{time::Duration, os::fd::RawFd};
  ///
  /// async fn write_example() -> std::io::Result<()> {
  ///     timeout(Duration::from_millis(10)).await?;
  ///     Ok(())
  /// }
  /// ```
  Timeout, fn timeout(duration: Duration) -> BufResult<i32, Vec<u8>>
);

impl_op!(
  "Create a soft-link",
  /// # Examples
  ///
  /// ```rust
  /// async fn write_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     // todo
  // ///     let (bytes_written, _buf) = lio::symlinkat(fd).await?;
  ///     Ok(())
  /// }
  /// ```
  SymlinkAt, fn symlinkat(new_dir_fd: RawFd, target: impl AsRef<Path>, linkpath: impl AsRef<Path>) -> io::Result<()> ; NulError
);

impl_op!(
  "Create a hard-link",
  /// # Examples
  ///
  /// ```rust
  /// async fn write_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///       // todo
  // ///     let (bytes_written, _buf) = lio::linkat(fd)?.await?;
  ///     Ok(())
  /// }
  /// ```
  LinkAt, fn linkat(old_dir_fd: RawFd, old_path: impl AsRef<Path>, new_dir_fd: RawFd, new_path: impl AsRef<Path>) -> io::Result<()> ; NulError
);

impl_op!(
  "Sync to fd.",
  /// # Examples
  ///
  /// ```rust
  /// async fn write_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     lio::fsync(fd).await?;
  ///     Ok(())
  /// }
  /// ```
  Fsync, fn fsync(fd: RawFd) -> io::Result<()>
);

impl_op!(
  "Performs a write operation on a file descriptor. Equivalent to the `pwrite` syscall.",
  /// # Examples
  ///
  /// ```rust
  /// async fn write_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let data = b"Hello, World!".to_vec();
  ///     let (result_bytes_written, _buf) = lio::write(fd, data, 0).await;
  ///     println!("Wrote {} bytes", result_bytes_written?);
  ///     Ok(())
  /// }
  /// ```
  Write, fn write(fd: RawFd, buf: Vec<u8>, offset: i64) -> BufResult<i32, Vec<u8>>
);

impl_op!(
  "Performs a read operation on a file descriptor. Equivalent of the `pread` syscall.",
  /// # Examples
  ///
  /// ```rust
  /// async fn read_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let mut buffer = vec![0u8; 1024];
  ///     let (res_bytes_read, buf) = lio::read(fd, buffer, 0).await;
  ///     let bytes_read = res_bytes_read?;
  ///     println!("Read {} bytes: {:?}", bytes_read, &buf[..bytes_read as usize]);
  ///     Ok(())
  /// }
  /// ```
  Read, fn read(fd: RawFd, mem: Vec<u8>, offset: i64) -> BufResult<i32, Vec<u8>>
);

impl_op!(
  "Truncates a file to a specified length.",
  /// # Examples
  ///
  /// ```rust
  /// async fn truncate_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     lio::truncate(fd, 1024).await?; // Truncate to 1KB
  ///     Ok(())
  /// }
  /// ```
  Truncate, fn truncate(fd: RawFd, len: u64) -> std::io::Result<()>
);

impl_op!(
  !detach
  "Creates a new socket with the specified domain, type, and protocol.",
  /// # Examples
  ///
  /// ```rust
  /// use socket2::{Domain, Type, Protocol};
  ///
  /// async fn socket_example() -> std::io::Result<()> {
  ///     let sock = lio::socket(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).await?;
  ///     println!("Created socket with fd: {}", sock);
  ///     Ok(())
  /// }
  /// ```
  Socket, fn socket(domain: socket2::Domain, ty: socket2::Type, proto: Option<socket2::Protocol>) -> std::io::Result<i32>
);

impl_op!(
  "Binds a socket to a specific address.",
  /// # Examples
  ///
  /// ```rust
  /// use socket2::SockAddr;
  ///
  /// async fn bind_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let addr = "127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap();
  ///     lio::bind(fd, addr).await?;
  ///     Ok(())
  /// }
  ///
  /// ```
  Bind, fn bind(fd: RawFd, addr: SocketAddr) -> std::io::Result<()>
);

impl_op!(
  "Accepts a connection on a listening socket.",
  /// # Examples
  ///
  /// ```rust
  /// use std::mem::MaybeUninit;
  ///
  /// async fn accept_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///
  ///     let (client_fd, addr) = lio::accept(fd).await?;
  ///     println!("Accepted connection on fd: {}", client_fd);
  ///     Ok(())
  /// }
  /// ```
  Accept, fn accept(fd: RawFd) -> std::io::Result<(RawFd, SocketAddr)>
);

impl_op!(
  "Marks a socket as listening for incoming connections.",
  /// # Examples
  ///
  /// ```rust
  /// use std::os::fd::RawFd;
  ///
  /// async fn listen_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     lio::listen(fd, 128).await?;
  ///     println!("Socket is now listening for connections");
  ///     Ok(())
  /// }
  /// ```
  Listen, fn listen(fd: RawFd, backlog: i32) -> std::io::Result<()>
);

impl_op!(
  "Connects a socket to a remote address.",
  /// # Examples
  ///
  /// ```rust
  /// use std::net::SocketAddr;
  ///
  /// async fn connect_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  ///     lio::connect(fd, addr).await?;
  ///     println!("Connected to remote address");
  ///     Ok(())
  /// }
  /// ```
  Connect, fn connect(fd: RawFd, addr: SocketAddr) -> std::io::Result<()>
);

impl_op!(
  "Sends data on a connected socket.",
  /// # Examples
  ///
  /// ```rust
  /// async fn send_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let data = b"Hello, server!".to_vec();
  ///     let (bytes_sent, _buf) = lio::send(fd, data, None).await;
  ///     println!("Sent {} bytes", bytes_sent?);
  ///     Ok(())
  /// }
  /// ```
  Send, fn send(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> BufResult<i32, Vec<u8>>
);

impl_op!(
  "Receives data from a connected socket.",
  /// # Examples
  ///
  /// ```rust
  /// async fn recv_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     let mut buffer = vec![0u8; 1024];
  ///     let (res_bytes_received, buf) = lio::recv(fd, buffer, None).await;
  ///     let bytes_received = res_bytes_received?;
  ///     println!("Received {} bytes: {:?}", bytes_received, &buf[..bytes_received as usize]);
  ///     Ok(())
  /// }
  /// ```
  Recv, fn recv(fd: RawFd, buf: Vec<u8>, flags: Option<i32>) -> BufResult<i32, Vec<u8>>
);

impl_op!(
  "Closes a file descriptor.",
  /// # Examples
  ///
  /// ```rust
  /// async fn close_example() -> std::io::Result<()> {
  ///     # let fd = 0;
  ///     lio::close(fd).await?;
  ///     println!("File descriptor closed successfully");
  ///     Ok(())
  /// }
  /// ```
  Close, fn close(fd: RawFd) -> io::Result<()>
);

impl_op!(
  "Opens a file relative to a directory file descriptor.",
  /// # Examples
  ///
  /// ```rust
  /// use std::ffi::CString;
  ///
  /// async fn openat_example() -> std::io::Result<()> {
  ///     let path = CString::new("/tmp/test.txt").unwrap();
  ///     let fd = lio::openat(libc::AT_FDCWD, path, libc::O_RDONLY).await?;
  ///     println!("Opened file with fd: {}", fd);
  ///     Ok(())
  /// }
  /// ```
  OpenAt, fn openat(fd: RawFd, path: CString, flags: i32) -> std::io::Result<i32>
);

#[cfg(linux)]
impl_op!(
  "Copies data between file descriptors without copying to userspace (Linux only).",
  /// This operation is only available on Linux systems with io_uring support.
  /// It's equivalent to the `tee(2)` system call.
  ///
  /// # Examples
  ///
  /// ```rust
  /// #[cfg(linux)]
  /// async fn tee_example() -> std::io::Result<()> {
  ///     # let fd_in = 0;
  ///     # let fd_out = 0;
  ///     let bytes_copied = lio::tee(fd_in, fd_out, 1024).await?;
  ///     println!("Copied {} bytes", bytes_copied);
  ///     Ok(())
  /// }
  /// ```
  Tee, fn tee(fd_in: RawFd, fd_out: RawFd, size: u32) -> std::io::Result<()>
);

/// Shut down the lio I/O driver background thread(s) and release OS resources.
///
/// After calling this, further I/O operations in this process are unsupported.
/// Calling shutdown more than once will panic.
pub fn exit() {
  Driver::shutdown()
}

// #[cfg(any(loom, test))]
#[doc(hidden)]
pub fn init() {
  let _ = Driver::get();
}

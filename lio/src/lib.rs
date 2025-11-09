//! # Lio - Platform-Independent Async I/O Library
//!
//! Lio is a high-performance, platform-independent async I/O library that provides
//! native support for the most efficient I/O mechanisms on each platform:
//!
//! - **Linux**: Uses [io_uring](https://man7.org/linux/man-pages/man7/io_uring.7.html) for maximum performance
//! - **Windows**: Uses [IOCP](https://docs.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports) (I/O Completion Ports)
//! - **macOS**: Uses [kqueue](https://man.openbsd.org/kqueue.2) for event notification
//!
//! ## Features
//!
//! - **Zero-copy operations** where possible
//! - **Async/await support** with standard Rust futures
//! - **Platform-specific optimizations** automatically selected
//! - **File I/O operations**: read, write, open, close, truncate
//! - **Network operations**: socket, bind, listen, accept, connect, send, recv
//! - **Automatic fallback** to blocking operations when async isn't supported
//!
//! ## **NOTE**
//! Currently this library is a bit finicky ([`libc::accept`] especially) on linux machines that doesn't support
//! io-uring operations, like wsl2. If anyone has a good idea of api design and detecting io-uring support on linux,
//! please file an issue.
//!
//! This problem arises when the library checks for the specific operation support, if yes
//! everything works. If no, it will call the blocking normal syscall. With accept, that means
//! blocking in a future which is really bad.
//!
//! ## Quick Start
//!
//! ```rust
//! use lio::{read, write, close};
//! use std::os::fd::RawFd;
//!
//! async fn example() -> std::io::Result<()> {
//!     let fd: RawFd = 1; // stdout
//!     let data = b"Hello, World!\n".to_vec();
//!     
//!     // Async write operation
//!     let (result, _buf) = write(fd, data, 0).await?;
//!     println!("Wrote {} bytes", result);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The library automatically selects the most efficient I/O mechanism:
//!
//! - On Linux with io_uring support, operations are submitted to the kernel's submission queue
//! - On other platforms, operations use polling-based async I/O with automatic fallback to blocking
//! - All operations return `OperationProgress<T>` which implements `Future<Output = io::Result<[different based on operation]>>`
//!
//! ## Platform Support
//!
//! | Platform | I/O Mechanism | Status |
//! |----------|---------------|---------|
//! | Linux    | io_uring      | ✅ Async IO support |
//! | Windows  | IOCP          | ✅ Full support |
//! | macOS    | kqueue        | ✅ event notification (kqueue) |
//! | Other Unix | poll/epoll   | ✅ event notification (epoll/poll/event ports) |
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
//!     let (bytes_written, _buf) = lio::write(fd, data, 0).await?;
//!     
//!     close(fd).await?;
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
//!     let sock = libc::socket(Domain::INET, Type::STREAM, Some(Protocol::TCP)).await?;
//!     let addr = socket2::SockAddr::from("127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap());
//!     
//!     libc::bind(sock, addr).await?;
//!     libc::listen(sock).await?;
//!     
//!     // Accept connections...
//!     Ok(())
//! }
//! ```
//!
//! ## Safety and Threading
//!
//! - All operations are safe and follow Rust's memory safety guarantees
//! - The library automatically handles thread management for background I/O processing
//! - Operations can be safely used across different threads
//! - Proper cleanup is guaranteed through Rust's drop semantics
//!
//! ## Error Handling
//!
//! All operations return `std::io::Result<T>` or `BufResult<T, B>` for operations
//! that return buffers. Errors are automatically converted from platform-specific
//! error codes to Rust's standard I/O error types.
//!
//! ## License
//!
//! This project is licensed under the MIT License - see the LICENSE file for details.

use std::{ffi::CString, mem::MaybeUninit, os::fd::RawFd};

/// Result type for operations that return both a result and a buffer.
///
/// This is commonly used for read/write operations where the buffer
/// is returned along with the operation result.
pub type BufResult<T, B> = (std::io::Result<T>, B);

#[macro_use]
pub(crate) mod macros;

mod driver;
pub mod loom;
pub mod op;
use op::*;
mod op_progress;
mod op_registration;

pub use op_progress::OperationProgress;

use socket2::SockAddr;

use crate::driver::Driver;

macro_rules! impl_op {
  (
    $(#[$($doc:tt)*])*
    $operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* )
  ) => {
    $(#[$($doc)*])*
    pub fn $name($($arg: $arg_ty),*) -> OperationProgress<$operation> {
      let operation_progress = Driver::submit(<$operation>::new($($arg),*));
      operation_progress
    }
  };
}

impl_op!(
    /// Performs an async write operation on a file descriptor. Equivalent to the `pwrite` syscall.
    ///
    /// # Arguments
    ///
    /// * `fd` - The file descriptor to write to
    /// * `buf` - The data buffer to write
    /// * `offset` - The file offset to write at (for regular files), cannot be < 0
    ///
    /// # Returns
    ///
    /// Return value implements `Future<BufResult<i32, Vec<u8>>>`.
    /// The result contains the number of bytes written and the original buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lio::write;
    /// use std::os::fd::RawFd;
    ///
    /// async fn write_example() -> std::io::Result<()> {
    ///     let fd: RawFd = 1; // stdout
    ///     let data = b"Hello, World!".to_vec();
    ///     let (bytes_written, _buf) = write(fd, data, 0).await?;
    ///     println!("Wrote {} bytes", bytes_written);
    ///     Ok(())
    /// }
    /// ```
    Write, fn write(fd: RawFd, buf: Vec<u8>, offset: i64)
);

impl_op!(
    /// Performs an async read operation on a file descriptor. Equivalent of the `pread` syscall.
    ///
    /// # Arguments
    ///
    /// * `fd` - The file descriptor to read from
    /// * `mem` - The buffer to read data into
    /// * `offset` - The file offset to read from (for regular files), cannot be < -1
    ///
    /// # Returns
    ///
    /// Returns `OperationProgress<Read>` which implements `Future<BufResult<i32, Vec<u8>>>`.
    /// The result contains the number of bytes read and the buffer with the data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lio::read;
    /// use std::os::fd::RawFd;
    ///
    /// async fn read_example() -> std::io::Result<()> {
    ///     let fd: RawFd = 0; // stdin
    ///     let mut buffer = vec![0u8; 1024];
    ///     let (bytes_read, buf) = read(fd, buffer, 0).await;
    ///     println!("Read {} bytes: {:?}", bytes_read?, &buf[..bytes_read as usize]);
    ///     Ok(())
    /// }
    /// ```
    Read, fn read(fd: RawFd, mem: Vec<u8>, offset: i64)
);

impl_op!(
    /// Truncates a file to a specified length.
    ///
    /// # Arguments
    ///
    /// * `fd` - The file descriptor to truncate
    /// * `len` - The new length of the file
    ///
    /// # Returns
    ///
    /// Returns `OperationProgress<Truncate>` which implements `Future<io::Result<()>>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lio::truncate;
    /// use std::os::fd::RawFd;
    ///
    /// async fn truncate_example() -> std::io::Result<()> {
    ///     let fd: RawFd = /* file descriptor */;
    ///     truncate(fd, 1024).await?; // Truncate to 1KB
    ///     Ok(())
    /// }
    /// ```
    Truncate, fn truncate(fd: RawFd, len: u64)
);

impl_op!(
    /// Creates a new socket with the specified domain, type, and protocol.
    ///
    /// # Arguments
    ///
    /// * `domain` - The socket domain (e.g., `Domain::INET` for IPv4)
    /// * `ty` - The socket type (e.g., `Type::STREAM` for TCP)
    /// * `proto` - The protocol (e.g., `Protocol::TCP`)
    ///
    /// # Returns
    ///
    /// Returns `OperationProgress<Socket>` which implements `Future<io::Result<RawFd>>`.
    /// The result is the file descriptor of the created socket.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lio::socket;
    /// use socket2::{Domain, Type, Protocol};
    ///
    /// async fn socket_example() -> std::io::Result<()> {
    ///     let sock = socket(Domain::INET, Type::STREAM, Some(Protocol::TCP)).await?;
    ///     println!("Created socket with fd: {}", sock);
    ///     Ok(())
    /// }
    /// ```
    Socket, fn socket(domain: socket2::Domain, ty: socket2::Type, proto: Option<socket2::Protocol>)
);

impl_op!(
  /// Binds a socket to a specific address.
  ///
  ///
  /// # Arguments
  ///
  /// * `fd` - The socket file descriptor
  /// * `addr` - The address to bind to
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Bind>` which implements `Future<io::Result<()>>`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::bind;
  /// use socket2::SockAddr;
  ///
  /// async fn bind_example() -> std::io::Result<()> {
  ///     let sock = /* socket fd */;
  ///     let addr = SockAddr::from("127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap());
  ///     bind(sock, addr).await?;
  ///     Ok(())
  /// }
  ///
  /// ```
  Bind, fn bind(fd: RawFd, addr: socket2::SockAddr)
);

impl_op!(
  /// Accepts a connection on a listening socket.
  ///
  /// **NOTE**: This operation doesn't seem to work on wsl2 linux. This is because they have a old
  /// kernel pinned.
  ///
  /// # Arguments
  ///
  /// * `fd` - The listening socket file descriptor
  /// * `addr` - Pointer to store the client's address
  /// * `len` - Pointer to store the address length
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Accept>` which implements `Future<io::Result<RawFd>>`.
  /// The result is the file descriptor of the accepted connection.
  ///
  /// # Safety
  ///
  /// The `addr` and `len` pointers must be valid and point to properly initialized memory.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::mem::MaybeUninit;
  /// use std::os::fd::RawFd;
  ///
  /// async fn accept_example() -> std::io::Result<()> {
  ///     let listen_fd: RawFd = /* listening socket */;
  ///     let mut addr_storage: MaybeUninit<socket2::SockAddrStorage> = MaybeUninit::uninit();
  ///     let mut addr_len = std::mem::size_of::<socket2::SockAddrStorage>() as libc::socklen_t;
  ///
  ///     let client_fd = lio::accept(listen_fd, addr_storage.as_mut_ptr(), &mut addr_len).await?;
  ///     println!("Accepted connection on fd: {}", client_fd);
  ///     Ok(())
  /// }
  /// ```
  Accept, fn accept(fd: RawFd)
);

impl_op!(
  /// Marks a socket as listening for incoming connections.
  ///
  /// # Arguments
  ///
  /// * `fd` - The socket file descriptor to mark as listening
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Listen>` which implements `Future<io::Result<()>>`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::listen;
  /// use std::os::fd::RawFd;
  ///
  /// async fn listen_example() -> std::io::Result<()> {
  ///     let sock: RawFd = /* socket fd */;
  ///     listen(sock).await?;
  ///     println!("Socket is now listening for connections");
  ///     Ok(())
  /// }
  /// ```
  Listen, fn listen(fd: RawFd, backlog: i32)
);

impl_op!(
  /// Connects a socket to a remote address.
  ///
  /// # Arguments
  ///
  /// * `fd` - The socket file descriptor
  /// * `addr` - The remote address to connect to
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Connect>` which implements `Future<io::Result<()>>`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::connect;
  /// use socket2::SockAddr;
  ///
  /// async fn connect_example() -> std::io::Result<()> {
  ///     let sock = /* socket fd */;
  ///     let addr = SockAddr::from("127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap());
  ///     let (bytes_written, _buf) = connect(sock, addr).await?;
  ///     println!("Connected to remote address");
  ///     Ok(())
  /// }
  /// ```
  Connect, fn connect(fd: RawFd, addr: SockAddr)
);

impl_op!(
  /// Sends data on a connected socket.
  ///
  /// # Arguments
  ///
  /// * `fd` - The socket file descriptor
  /// * `buf` - The data buffer to send
  /// * `flags` - Optional socket send flags
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Send>` which implements `Future<BufResult<i32, Vec<u8>>>`.
  /// The result contains the number of bytes sent and the original buffer.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::send;
  /// use std::os::fd::RawFd;
  ///
  /// async fn send_example() -> std::io::Result<()> {
  ///     let sock: RawFd = /* connected socket */;
  ///     let data = b"Hello, server!".to_vec();
  ///     let (bytes_sent, _buf) = send(sock, data, None).await?;
  ///     println!("Sent {} bytes", bytes_sent);
  ///     Ok(())
  /// }
  /// ```
  Send, fn send(fd: RawFd, buf: Vec<u8>, flags: Option<i32>)
);

impl_op!(
  /// Receives data from a connected socket.
  ///
  /// # Arguments
  ///
  /// * `fd` - The socket file descriptor
  /// * `buf` - The buffer to receive data into
  /// * `flags` - Optional socket receive flags
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Recv>` which implements `Future<BufResult<i32, Vec<u8>>>`.
  /// The result contains the number of bytes received and the buffer with the data.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::recv;
  /// use std::os::fd::RawFd;
  ///
  /// async fn recv_example() -> std::io::Result<()> {
  ///     let sock: RawFd = /* connected socket */;
  ///     let mut buffer = vec![0u8; 1024];
  ///     let (bytes_received, buf) = recv(sock, buffer, None).await?;
  ///     println!("Received {} bytes: {:?}", bytes_received, &buf[..bytes_received as usize]);
  ///     Ok(())
  /// }
  /// ```
  Recv, fn recv(fd: RawFd, buf: Vec<u8>, flags: Option<i32>)
);

impl_op!(
  /// Closes a file descriptor.
  ///
  /// # Arguments
  ///
  /// * `fd` - The file descriptor to close
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Close>` which implements `Future<io::Result<()>>`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::close;
  /// use std::os::fd::RawFd;
  ///
  /// async fn close_example() -> std::io::Result<()> {
  ///     let fd: RawFd = /* file descriptor */;
  ///     close(fd).await?;
  ///     println!("File descriptor closed successfully");
  ///     Ok(())
  /// }
  /// ```
  Close, fn close(fd: RawFd)
);

impl_op!(
  /// Opens a file relative to a directory file descriptor.
  ///
  /// # Arguments
  ///
  /// * `fd` - The directory file descriptor (use `libc::AT_FDCWD` for current directory)
  /// * `path` - The path to the file to open
  /// * `flags` - The open flags (e.g., `O_RDONLY`, `O_WRONLY`, `O_CREAT`)
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<OpenAt>` which implements `Future<io::Result<RawFd>>`.
  /// The result is the file descriptor of the opened file.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use lio::openat;
  /// use std::ffi::CString;
  ///
  /// async fn openat_example() -> std::io::Result<()> {
  ///     let path = CString::new("/tmp/test.txt").unwrap();
  ///     let fd = openat(libc::AT_FDCWD, path, libc::O_RDONLY).await?;
  ///     println!("Opened file with fd: {}", fd);
  ///     Ok(())
  /// }
  /// ```
  OpenAt, fn openat(fd: RawFd, path: CString, flags: i32)
);

#[cfg(linux)]
impl_op!(
  /// Copies data between file descriptors without copying to userspace (Linux only).
  ///
  /// This operation is only available on Linux systems with io_uring support.
  /// It's equivalent to the `tee(2)` system call.
  ///
  /// # Arguments
  ///
  /// * `fd_in` - The input file descriptor
  /// * `fd_out` - The output file descriptor
  /// * `size` - The number of bytes to copy
  ///
  /// # Returns
  ///
  /// Returns `OperationProgress<Tee>` which implements `Future<io::Result<u32>>`.
  /// The result is the number of bytes actually copied.
  ///
  /// # Examples
  ///
  /// ```rust
  /// #[cfg(linux)]
  /// async fn tee_example() -> std::io::Result<()> {
  ///     use lio::tee;
  ///     let fd_in: RawFd = /* input file descriptor */;
  ///     let fd_out: RawFd = /* output file descriptor */;
  ///     let bytes_copied = tee(fd_in, fd_out, 1024).await?;
  ///     println!("Copied {} bytes", bytes_copied);
  ///     Ok(())
  /// }
  /// ```
  Tee, fn tee(fd_in: RawFd, fd_out: RawFd, size: u32)
);

// #[cfg(linux)]
// pub fn tick() {
//   let driver = Driver::get();
//   if driver
//     .0
//     .has_done_work
//     .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
//     .is_ok()
//   {
//     let _ = driver.0.inner.submit();
//   }
// }

/// Shut down the lio I/O driver background thread(s) and release OS resources.
///
/// After calling this, further I/O operations in this process are unsupported.
/// Calling shutdown more than once will panic.
pub fn shutdown() {
  Driver::shutdown()
}

pub fn init() {
  let _ = Driver::get();
}

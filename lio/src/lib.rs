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

// TODO: Safe shutdown
use std::thread::JoinHandle;
use std::{
  collections::HashMap,
  ffi::CString,
  io,
  mem::MaybeUninit,
  os::fd::RawFd,
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  task::Waker,
};
#[cfg(linux)]
use std::{mem, sync::atomic::AtomicBool};

/// Result type for operations that return both a result and a buffer.
///
/// This is commonly used for read/write operations where the buffer
/// is returned along with the operation result.
pub type BufResult<T, B> = (std::io::Result<T>, B);

#[macro_use]
pub(crate) mod macros;

pub mod op;
mod op_progress;
mod op_registration;
pub use op_progress::OperationProgress;

#[cfg(linux)]
use io_uring::{IoUring, cqueue::Entry};

#[cfg(not_linux)]
use polling::Poller;
use socket2::{SockAddr, SockAddrStorage};

use crate::op_registration::OpRegistration;
#[cfg(linux)]
use crate::op_registration::OpRegistrationStatus;

struct Driver(Arc<DriverInner>);

struct DriverInner {
  #[cfg(linux)]
  inner: IoUring,
  #[cfg(linux)]
  has_done_work: AtomicBool,
  #[cfg(linux)]
  submission_guard: Mutex<()>,

  #[cfg(not_linux)]
  poller: polling::Poller,

  wakers: Mutex<HashMap<u64, OpRegistration>>,
  // Shared shutdown state and background thread handle
  shutting_down: AtomicBool,
  background_handle: Mutex<Option<JoinHandle<()>>>,
}

/// Performs an async write operation on a file descriptor.
///
/// # Arguments
///
/// * `fd` - The file descriptor to write to
/// * `buf` - The data buffer to write
/// * `offset` - The file offset to write at (for regular files)
///
/// # Returns
///
/// Returns `OperationProgress<Write>` which implements `Future<BufResult<i32, Vec<u8>>>`.
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
pub fn write(
  fd: RawFd,
  buf: Vec<u8>,
  offset: u64,
) -> op_progress::OperationProgress<op::Write> {
  let op = <op::Write>::new(fd, buf, offset);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

/// Performs an async read operation on a file descriptor.
///
/// # Arguments
///
/// * `fd` - The file descriptor to read from
/// * `mem` - The buffer to read data into
/// * `offset` - The file offset to read from (for regular files)
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
pub fn read(
  fd: RawFd,
  mem: Vec<u8>,
  offset: u64,
) -> op_progress::OperationProgress<op::Read> {
  let op = <op::Read>::new(fd, mem, offset);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

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
pub fn truncate(
  fd: RawFd,
  len: u64,
) -> op_progress::OperationProgress<op::Truncate> {
  let op = <op::Truncate>::new(fd, len);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

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
pub fn socket(
  domain: socket2::Domain,
  ty: socket2::Type,
  proto: Option<socket2::Protocol>,
) -> op_progress::OperationProgress<op::Socket> {
  let op = <op::Socket>::new(domain, ty, proto);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

/// Binds a socket to a specific address.
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
/// ```
pub fn bind(
  fd: RawFd,
  addr: socket2::SockAddr,
) -> op_progress::OperationProgress<op::Bind> {
  let op = <op::Bind>::new(fd, addr);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

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
pub fn accept(
  fd: RawFd,
  addr: *mut MaybeUninit<SockAddrStorage>,
  len: *mut libc::socklen_t,
) -> op_progress::OperationProgress<op::Accept> {
  let op = <op::Accept>::new(fd, addr, len);

  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_poll(fd, PollInterest::Read, op)
    }
  }
}

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
pub fn listen(fd: RawFd) -> op_progress::OperationProgress<op::Listen> {
  let op = <op::Listen>::new(fd);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

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
pub fn connect(
  fd: RawFd,
  addr: SockAddr,
) -> op_progress::OperationProgress<op::Connect> {
  let op = <op::Connect>::new(fd, addr);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_poll(fd, PollInterest::Write, op)
    }
  }
}

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
pub fn send(
  fd: RawFd,
  buf: Vec<u8>,
  flags: Option<i32>,
) -> op_progress::OperationProgress<op::Send> {
  let op = <op::Send>::new(fd, buf, flags);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_poll(fd, PollInterest::Write, op)
    }
  }
}

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
pub fn recv(
  fd: RawFd,
  buf: Vec<u8>,
  flags: Option<i32>,
) -> op_progress::OperationProgress<op::Recv> {
  let op = <op::Recv>::new(fd, buf, flags);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_poll(fd, PollInterest::Read, op)
    }
  }
}

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
pub fn close(fd: RawFd) -> op_progress::OperationProgress<op::Close> {
  let op = <op::Close>::new(fd);
  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

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
pub fn openat(
  fd: RawFd,
  path: CString,
  flags: i32,
) -> op_progress::OperationProgress<op::OpenAt> {
  let op = <op::OpenAt>::new(fd, path, flags);

  cfg_if::cfg_if! {
    if #[cfg(linux)] {
      Driver::submit(op)
    } else {
      Driver::submit_block(op)
    }
  }
}

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
#[cfg(linux)]
pub fn tee(
  fd_in: RawFd,
  fd_out: RawFd,
  size: u32,
) -> op_progress::OperationProgress<op::Tee> {
  let op = <op::Tee>::new(fd_in, fd_out, size);
  Driver::submit(op)
}

#[cfg(linux)]
pub fn tick() {
  let driver = Driver::get();
  if driver
    .0
    .has_done_work
    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
    .is_ok()
  {
    let _ = driver.0.inner.submit();
  }
}

#[cfg(not_linux)]
pub fn tick() {}

#[cfg(not_linux)]
#[derive(Clone, Copy)]
pub(crate) enum PollInterest {
  Read,
  Write,
}

#[cfg(not_linux)]
impl PollInterest {
  fn as_event(self, id: u64) -> polling::Event {
    polling::Event::new(
      id as usize,
      matches!(self, PollInterest::Read),
      matches!(self, PollInterest::Write),
    )
  }
}

impl Driver {
  fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::AcqRel)
  }

  pub(crate) fn get() -> &'static Driver {
    static DRIVER: OnceLock<Driver> = OnceLock::new();

    DRIVER.get_or_init(|| {
      let driver = Driver(Arc::new(DriverInner {
        #[cfg(linux)]
        inner: IoUring::new(256).unwrap(),
        #[cfg(linux)]
        submission_guard: Mutex::new(()),
        #[cfg(linux)]
        has_done_work: AtomicBool::new(false),

        #[cfg(not_linux)]
        poller: polling::Poller::new().unwrap(),

        wakers: Mutex::new(HashMap::default()),
        shutting_down: AtomicBool::new(false),
        background_handle: Mutex::new(None),
      }));

      driver.background();

      driver
    })
  }

  pub(crate) fn detach(&self, id: u64) -> Option<()> {
    let mut _lock = Driver::get().0.wakers.lock().unwrap();

    cfg_if::cfg_if! {
      if #[cfg(linux)] {
        let thing = _lock.get_mut(&id)?;
        thing.status = OpRegistrationStatus::Cancelling;
      } else {
       let thing = _lock.remove(&id)?;
        // If exists:

       // SAFETY: Just turning a RawFd into something polling crate can understand.
       let fd = unsafe {
         use std::os::fd::BorrowedFd;
         BorrowedFd::borrow_raw(thing.fd())
       };
       self.0.poller.delete(fd).unwrap();
      }
    }

    Some(())
  }
}

#[cfg(not_linux)]
impl Driver {
  pub(crate) fn insert_poll(&self, fd: RawFd, interest: PollInterest) -> u64 {
    let mut _lock = self.0.wakers.lock().unwrap();
    let id = Self::next_id();

    let op = OpRegistration::new(fd, interest);
    let _ = _lock.insert(id, op);

    // SAFETY: Just turning a RawFd into something polling crate can understand.
    unsafe {
      use std::os::fd::BorrowedFd;

      let fd = BorrowedFd::borrow_raw(fd);
      self.0.poller.add(&fd, interest.as_event(id)).unwrap();
    };

    id
  }
  pub(crate) fn submit_block<O: op::Operation>(op: O) -> OperationProgress<O> {
    OperationProgress::new_blocking(op)
  }
  pub(crate) fn submit_poll<O: op::Operation>(
    fd: RawFd,
    interest: PollInterest,
    op: O,
  ) -> OperationProgress<O> {
    let id = Driver::get().insert_poll(fd, interest);
    OperationProgress::new_poll(id, op)
  }

  pub(crate) fn register_repoll(
    &self,
    key: u64,
    waker: Waker,
  ) -> Option<io::Result<()>> {
    let mut _lock = self.0.wakers.lock().unwrap();
    let thing = _lock.get_mut(&key)?;

    let fd = unsafe {
      use std::os::fd::BorrowedFd;
      BorrowedFd::borrow_raw(thing.fd())
    };

    if let Err(err) = self.0.poller.modify(fd, thing.interest().as_event(key)) {
      return Some(Err(err));
    };

    thing.set_waker(waker);

    Some(Ok(()))
  }

  pub fn background(&self) {
    let driver = self.0.clone();
    let handle = utils::create_worker(move || {
      let mut events = polling::Events::new();
      loop {
        if driver.shutting_down.load(Ordering::Acquire) {
          break;
        }
        events.clear();
        driver.poller.wait(&mut events, None).unwrap();

        if driver.shutting_down.load(Ordering::Acquire) {
          break;
        }

        let mut _lock = driver.wakers.lock().unwrap();
        for event in events.iter() {
          if let Some(reg) = _lock.get_mut(&(event.key as _)) {
            reg.wake();
          }
        }
      }
    });

    *self.0.background_handle.lock().unwrap() = Some(handle);
  }
}

#[cfg(linux)]
impl Driver {
  pub fn background(&self) {
    // SAFETY: completion_shared is only accessed here so it's a singlethreaded access, hence
    // guaranteed only to have one completion queue.
    let driver = self.0.clone();
    let handle = utils::create_worker(move || {
      loop {
        if driver.shutting_down.load(Ordering::Acquire) {
          break;
        }
        driver.inner.submit_and_wait(1).unwrap();

        let entries: Vec<Entry> =
            // SAFETY: The only thread that is concerned with completion queue.
            unsafe { driver.inner.completion_shared() }.collect();

        for entry in entries {
          let operation_id = entry.user_data();

          let mut wakers = driver.wakers.lock().unwrap();

          // If the operation id is not registered (e.g., wake-up NOP), skip.
          let Some(op_registration) = wakers.get_mut(&operation_id) else {
            continue;
          };

          let old_value = mem::replace(
            &mut op_registration.status,
            OpRegistrationStatus::Done { ret: entry.result() },
          );
          let waker: Option<Waker> = match old_value {
            OpRegistrationStatus::Waiting { ref registered_waker } => {
              registered_waker.take()
            }
            OpRegistrationStatus::Cancelling => {
              let reg = wakers.remove(&operation_id).unwrap();

              // Dropping the operation.
              (reg.drop_fn)(reg.op);

              None
            }
            OpRegistrationStatus::Done { .. } => {
              unreachable!("already processed entry");
            }
          };

          if let Some(waker) = waker {
            waker.wake();
          };
        }
        unsafe { driver.inner.completion_shared() }.sync();
      }
    });

    *self.0.background_handle.lock().unwrap() = Some(handle);
  }
  fn submit<T>(op: T) -> OperationProgress<T>
  where
    T: op::Operation,
  {
    //if T::supported() {
    let operation_id = Self::get().push::<T>(op);
    OperationProgress::<T>::new_uring(operation_id)
    //} else {
    //  OperationProgress::<T>::new_blocking(op)
    //}
  }

  fn push<T: op::Operation>(&self, op: T) -> u64 {
    let operation_id = Self::next_id();
    let entry = op.create_entry().user_data(operation_id);

    let mut _lock = self.0.wakers.lock().unwrap();

    // SAFETY: because of references rules, a "fake" lock has to be implemented here, but because
    // of it, this is safe.
    let _g = self.0.submission_guard.lock();
    unsafe {
      let mut sub = self.0.inner.submission_shared();
      sub.push(&entry).expect("unwrapping for now");
      sub.sync();
      drop(sub);
    }
    drop(_g);

    _lock.insert(operation_id, OpRegistration::new(op));

    self.0.has_done_work.store(true, Ordering::SeqCst);

    operation_id
  }

  fn check_registration<T: op::Operation>(
    &self,
    id: u64,
    waker: Waker,
  ) -> Option<CheckRegistrationResult<T::Result>> {
    let mut _lock = self.0.wakers.lock().unwrap();
    let op_registration = _lock.get_mut(&id)?;

    Some(match op_registration.status {
      OpRegistrationStatus::Done { ret } => {
        let op_registration = _lock.remove(&id).expect("what");

        // SAFETY: The pointer was created with Box::into_raw in queue_submit with a concrete type T
        // We can safely cast it back to the concrete type T
        let mut value = unsafe { Box::from_raw(op_registration.op as *mut T) };

        let raw_ret = if ret < 0 {
          Err(io::Error::from_raw_os_error(-ret))
        } else {
          Ok(ret)
        };

        CheckRegistrationResult::Value(value.result(raw_ret))
      }
      OpRegistrationStatus::Waiting { ref mut registered_waker } => {
        registered_waker.replace(Some(waker));
        CheckRegistrationResult::WakerSet
      }
      OpRegistrationStatus::Cancelling => {
        unreachable!("wtf to do here?");
      }
    })
  }
}

#[cfg(linux)]
pub(crate) enum CheckRegistrationResult<V> {
  /// Waker has been registered and future should return Poll::Pending
  WakerSet,
  /// Value has been returned and future should poll anymore.
  Value(V),
}

mod utils {
  use std::thread::{self, JoinHandle};

  pub fn create_worker<F, T>(handle: F) -> JoinHandle<T>
  where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
  {
    thread::Builder::new()
      .name("lio".into())
      .spawn(handle)
      .expect("failed to launch the worker thread")
  }
}

/// Shut down the lio I/O driver background thread(s) and release OS resources.
///
/// After calling this, further I/O operations in this process are unsupported.
/// Calling shutdown more than once will panic.
pub fn shutdown() {
  static DONE_BEFORE: OnceLock<()> = OnceLock::new();
  if DONE_BEFORE.get().is_some() {
    panic!("shutdown after shutdown");
  }

  let driver = Driver::get();
  driver.0.shutting_down.store(true, Ordering::Release);

  cfg_if::cfg_if! {
    if #[cfg(not_linux)] {
      // Wake the poller so it can observe the shutdown flag
      let _ = driver.0.poller.notify();
    } else {
      // Submit a NOP to wake submit_and_wait
      unsafe {
        let _g = driver.0.submission_guard.lock();
        let mut sub = driver.0.inner.submission_shared();
        let entry = io_uring::opcode::Nop::new().build().user_data(0);
        let _ = sub.push(&entry);
        sub.sync();
        drop(sub);
      }
      let _ = driver.0.inner.submit();
    }
  }

  if let Some(handle) = driver.0.background_handle.lock().unwrap().take() {
    let _ = handle.join();
  }

  let _ = DONE_BEFORE.set(());
}

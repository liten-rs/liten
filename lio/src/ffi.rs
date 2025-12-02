//! # `lio` C API
//!
//! ## Compiling
//! `lio` can be compiled using cargo with command:
//! ```sh
//! make lio-cbuild
//! ```
//! `lio` dynamic library can be found at `target/release/liblio.{dylib,dll,so}`
#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[cfg(not(lio_unstable_ffi))]
compile_error!(
  "\
    The `ffi` feature is unstable, and requires the \
    `RUSTFLAGS='--cfg lio_unstable_ffi'` environment variable to be set.\
"
);

use std::ptr;

use crate::op::net_utils::{self, sockaddr_to_socketaddr};

#[unsafe(no_mangle)]
pub extern "C" fn lio_shutdown(
  fd: libc::c_int,
  how: i32,
  callback: extern "C" fn(i32),
) {
  crate::shutdown(fd, how).when_done(move |res| {
    let result_code = match res {
      Ok(_) => 0,
      Err(err) => err.raw_os_error().unwrap_or(-1),
    };
    callback(result_code);
  });
}

// TODO
#[allow(unused)]
#[unsafe(no_mangle)]
pub extern "C" fn lio_symlinkat(
  new_dir_fd: libc::c_int,
  target: *const libc::c_char,
  linkpath: *const libc::c_char,
  callback: extern "C" fn(i32),
) {
  todo!();
  // crate::symlinkat(new_dir_fd, target.).when_done(move |res| {
  //   let result_code = match res {
  //     Ok(_) => 0,
  //     Err(err) => err.raw_os_error().unwrap_or(-1),
  //   };
  //   callback(result_code);
  // });
}

// TODO
#[allow(unused)]
#[unsafe(no_mangle)]
pub extern "C" fn lio_linkat(
  old_dir_fd: libc::c_int,
  old_path: *const libc::c_char,
  new_dir_fd: libc::c_int,
  new_path: *const libc::c_char,
  callback: extern "C" fn(i32),
) {
  todo!();
  // crate::linkat(new_dir_fd, target.).when_done(move |res| {
  //   let result_code = match res {
  //     Ok(_) => 0,
  //     Err(err) => err.raw_os_error().unwrap_or(-1),
  //   };
  //   callback(result_code);
  // });
}

#[unsafe(no_mangle)]
pub extern "C" fn lio_fsync(fd: libc::c_int, callback: extern "C" fn(i32)) {
  crate::fsync(fd).when_done(move |res| {
    let result_code = match res {
      Ok(_) => 0,
      Err(err) => err.raw_os_error().unwrap_or(-1),
    };
    callback(result_code);
  });
}

// write todo
// read todo

#[unsafe(no_mangle)]
pub extern "C" fn lio_truncate(
  fd: libc::c_int,
  len: u64,
  callback: extern "C" fn(i32),
) {
  crate::truncate(fd, len).when_done(move |res| {
    let result_code = match res {
      Ok(_) => 0,
      Err(err) => err.raw_os_error().unwrap_or(-1),
    };
    callback(result_code);
  });
}

#[unsafe(no_mangle)]
pub extern "C" fn lio_socket(
  domain: i32,
  ty: i32,
  proto: i32,
  callback: extern "C" fn(i32),
) {
  crate::socket(domain.into(), ty.into(), Some(proto.into())).when_done(
    move |res| {
      let result_code = match res {
        Ok(fd) => fd,
        Err(err) => err.raw_os_error().unwrap_or(-1),
      };
      callback(result_code);
    },
  );
}

// Safety: C sucks man
#[unsafe(no_mangle)]
pub extern "C" fn lio_bind(
  fd: libc::c_int,
  sock: *const libc::sockaddr,
  sock_len: *const libc::socklen_t,
  callback: extern "C" fn(i32),
) {
  // TODO: fix unwrap.
  let addr = sockaddr_to_socketaddr(sock, unsafe { *sock_len }).unwrap();
  // TODO: Optimise
  crate::bind(fd, addr).when_done(move |res| {
    let result_code = match res {
      Ok(_) => 0,
      Err(err) => err.raw_os_error().unwrap_or(-1),
    };
    callback(result_code);
  });
}

/// ptr is null if operation fails.
#[unsafe(no_mangle)]
pub extern "C" fn lio_accept(
  fd: libc::c_int,
  callback: extern "C" fn(i32, *const libc::sockaddr_storage),
) {
  // TODO: fix unwrap.
  crate::accept(fd).when_done(move |res| {
    let (res, addr) = match res {
      Ok((fd, addr)) => (
        fd,
        Box::into_raw(Box::new(net_utils::std_socketaddr_into_libc(addr)))
          as *const _,
      ),
      Err(err) => (err.raw_os_error().unwrap_or(-1), ptr::null()),
    };

    callback(res, addr)
  });
}

/// Hello
#[unsafe(no_mangle)]
pub extern "C" fn lio_listen(
  fd: libc::c_int,
  backlog: i32,
  callback: extern "C" fn(i32),
) {
  crate::listen(fd, backlog).when_done(move |res| {
    let result_code = match res {
      Ok(_) => 0,
      Err(err) => err.raw_os_error().unwrap_or(-1),
    };
    callback(result_code);
  });
}

// send: TODO
// recv: TODO

#[unsafe(no_mangle)]
pub extern "C" fn lio_close(fd: libc::c_int, callback: extern "C" fn(i32)) {
  crate::close(fd).when_done(move |res| {
    let result_code = match res {
      Ok(_) => 0,
      Err(err) => err.raw_os_error().unwrap_or(-1),
    };
    callback(result_code);
  });
}

// openat: TODO

#[unsafe(no_mangle)]
pub extern "C" fn lio_exit() {
  crate::exit()
}

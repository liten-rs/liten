use std::{
  ffi::{CString, NulError},
  os::{fd::RawFd, unix::ffi::OsStringExt},
  path::Path,
};

#[cfg(linux)]
use io_uring::types::Fd;

#[cfg(not(linux))]
use crate::op::EventType;

use super::Operation;

pub struct LinkAt {
  old_dir_fd: RawFd,
  old_path: CString,
  new_dir_fd: RawFd,
  new_path: CString,
}

// TODO: test
impl LinkAt {
  pub(crate) fn new(
    old_dir_fd: RawFd,
    old_path: impl AsRef<Path>,
    new_dir_fd: RawFd,
    new_path: impl AsRef<Path>,
  ) -> Result<Self, NulError> {
    let old_path_osstr = old_path.as_ref().as_os_str().to_os_string();
    let new_path_osstr = new_path.as_ref().as_os_str().to_os_string();
    Ok(Self {
      old_dir_fd,
      old_path: CString::new(old_path_osstr.into_vec())?,
      new_dir_fd,
      new_path: CString::new(new_path_osstr.into_vec())?,
    })
  }
}

impl Operation for LinkAt {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 39;

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::LinkAt::new(
      Fd(self.old_dir_fd),
      self.old_path.as_ptr(),
      Fd(self.new_dir_fd),
      self.new_path.as_ptr(),
    )
    .build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(linkat(
      self.old_dir_fd,
      self.old_path.as_ptr(),
      self.new_dir_fd,
      self.new_path.as_ptr(),
      0
    ))
  }
}

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

pub struct SymlinkAt {
  fd: RawFd,
  target: CString,
  linkpath: CString,
}

// TODO: test
impl SymlinkAt {
  pub(crate) fn new(
    new_dir_fd: RawFd,
    target: impl AsRef<Path>,
    linkpath: impl AsRef<Path>,
  ) -> Result<Self, NulError> {
    let target = target.as_ref().as_os_str().to_os_string();
    let linkpath = linkpath.as_ref().as_os_str().to_os_string();
    Ok(Self {
      fd: new_dir_fd,
      target: CString::new(target.into_vec())?,
      linkpath: CString::new(linkpath.into_vec())?,
    })
  }
}

impl Operation for SymlinkAt {
  impl_result!(());

  #[cfg(linux)]
  const OPCODE: u8 = 38;

  #[cfg(not(linux))]
  const EVENT_TYPE: Option<EventType> = None;

  #[cfg(not(linux))]
  fn fd(&self) -> Option<RawFd> {
    None
  }

  #[cfg(linux)]
  fn create_entry(&self) -> io_uring::squeue::Entry {
    io_uring::opcode::SymlinkAt::new(
      Fd(self.fd),
      self.target.as_ptr(),
      self.linkpath.as_ptr(),
    )
    .build()
  }

  fn run_blocking(&self) -> std::io::Result<i32> {
    syscall!(symlinkat(self.target.as_ptr(), self.fd, self.linkpath.as_ptr()))
  }
}

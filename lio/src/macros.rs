macro_rules! impl_result {
  (()) => {
    type Output = ();
    type Result = std::io::Result<Self::Output>;

    fn result(&mut self, res: std::io::Result<i32>) -> Self::Result {
      res.map(|code| {
        assert!(code == 0);
      })
    }
  };

  (fd) => {
    #[cfg(unix)]
    type Output = std::os::fd::RawFd;
    #[cfg(unix)]
    type Result = std::io::Result<Self::Output>;

    /// File descriptor returned from the operation.
    fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
      fd
    }
  };
}

macro_rules! impl_no_readyness {
  () => {
    #[cfg(not(linux))]
    const EVENT_TYPE: Option<crate::op::EventType> = None;

    #[cfg(not(linux))]
    fn fd(&self) -> Option<std::os::fd::RawFd> {
      None
    }
  };
}

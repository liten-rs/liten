macro_rules! impl_result {
  (()) => {
    // #[cfg(unix)]
    type Output = ();
    // #[cfg(unix)]
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

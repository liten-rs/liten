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
    type Output = std::os::fd::RawFd;
    type Result = std::io::Result<Self::Output>;

    /// File descriptor returned from the operation.
    fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
      fd
    }
  };
}

macro_rules! impl_op {
  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* )) => {
    #[allow(dead_code)]
    pub fn $name($($arg: $arg_ty),*) -> op_progress::OperationProgress<$operation> {
      Driver::submit(<$operation>::new($($arg),*))
    }
  };
}

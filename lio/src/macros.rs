macro_rules! impl_result {
  (()) => {
    #[cfg(unix)]
    type Output = ();
    #[cfg(unix)]
    type Result = std::io::Result<Self::Output>;

    #[cfg(target_os = "linux")]
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
    #[cfg(target_os = "linux")]
    fn result(&mut self, fd: std::io::Result<i32>) -> Self::Result {
      fd
    }
  };
}

macro_rules! impl_op {
  ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* )) => {
    pub fn $name($($arg: $arg_ty),*) -> op_progress::OperationProgress<$operation> {
      let op = <$operation>::new($($arg),*);
      // if <$operation>::supported() {
      Driver::submit(op)
      // } else {
      //   let ret = op.run_blocking();
      //   op.result(ret)
      // }
    }
  };
}

// macro_rules! impl_op {
//   ($operation:ty, fn $name:ident ( $($arg:ident: $arg_ty:ty),* )) => {
//     pub fn $name($($arg: $arg_ty),*) -> op_progress::OperationProgress<$operation> {
//       let op = <$operation>::new($($arg),*);
//       if <$operation>::supported() {
//       }
//       Driver::submit(op)
//     }
//   };
// }

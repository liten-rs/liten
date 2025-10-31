pub mod sync {
  #[cfg(lio_shuttle)]
  pub use shuttle::sync::*;
  #[cfg(not(lio_shuttle))]
  pub use std::sync::*;

  pub use std::sync::OnceLock;
}

#[cfg(lio_shuttle)]
pub use shuttle::thread;
#[cfg(not(lio_shuttle))]
pub use std::thread;

pub mod test_utils {
  #[cfg(lio_shuttle)]
  pub use shuttle::future::spawn;

  #[cfg(not(lio_shuttle))]
  pub use tokio::task::spawn;

  pub fn block_on<F, O>(fut: F) -> O
  where
    F: Future<Output = O>,
  {
    #[cfg(not(lio_shuttle))]
    let ret = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(fut);

    #[cfg(lio_shuttle)]
    let ret = shuttle::future::block_on(fut);

    ret
  }
}

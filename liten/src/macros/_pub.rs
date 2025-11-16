/// A macro for joining multiple futures together.
///
/// This macro takes a list of futures and returns a new future that completes
/// when all of the input futures complete. The result is a tuple containing
/// the outputs of all the futures in the same order they were provided.
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// use liten::join;
///
/// #[liten::main]
/// async fn main() {
///     let future1 = async { 1 };
///     let future2 = async { "hello" };
///     let future3 = async { true };
///
///     let (a, b, c) = join!(future1, future2, future3);
///     assert_eq!(a, 1);
///     assert_eq!(b, "hello");
///     assert_eq!(c, true);
/// }
/// ```
///
/// The futures don't have to return the same type.
///
/// # Notes
///
/// - All futures are polled concurrently, so they make progress simultaneously
/// - The macro returns a tuple with the same number of elements as input futures
/// - If any future panics, the panic will be propagated
/// - The macro is zero-cost and has minimal runtime overhead
#[macro_export]
macro_rules! join {
    (@ {
        // One `_` for each branch in the `join!` macro. This is not used once
        // normalization is complete.
        ( $($count:tt)* )

        // Normalized join! branches
        $( ( $($skip:tt)* ) $e:expr, )*

    }) => {{
        use $crate::macros::_pub::maybe_done::{maybe_done};
        use std::{
          pin::Pin,
          future::{Future, poll_fn},
          task::Poll::{Ready, Pending}
        };

        // Safety: nothing must be moved out of `futures`. This is to satisfy
        // the requirement of `Pin::new_unchecked` called below.
        let mut futures = ( $( maybe_done($e), )* );

        poll_fn(move |cx| {
            let mut is_pending = false;

            $(
                // Extract the future for this branch from the tuple.
                let ( $($skip,)* fut, .. ) = &mut futures;

                // Safety: future is stored on the stack above
                // and never moved.
                let fut = unsafe { Pin::new_unchecked(fut) };

                // Try polling
                if fut.poll(cx).is_pending() {
                    is_pending = true;
                }
            )*

            if is_pending {
                Pending
            } else {
                Ready(($({
                    // Extract the future for this branch from the tuple.
                    let ( $($skip,)* fut, .. ) = &mut futures;

                    // Safety: future is stored on the stack above
                    // and never moved.
                    let fut = unsafe { Pin::new_unchecked(fut) };

                    fut.take_output().expect("expected completed future")
                },)*))
            }
        }).await
    }};

    // ===== Normalize =====

    (@ { ( $($s:tt)* ) $($t:tt)* } $e:expr, $($r:tt)* ) => {
        $crate::join!(@{ ($($s)* _) $($t)* ($($s)*) $e, } $($r)*)
    };

    // ===== Entry point =====

    ( $($e:expr),* $(,)?) => {
        $crate::join!(@{ () } $($e,)*)
    };
}

#[crate::internal_test]
#[cfg(feature = "runtime")]
fn testing() {
  use std::future::ready;
  crate::runtime::Runtime::single_threaded().block_on(async {
    let _result = join!(ready(3u8), ready(""));
  })
}

pub mod maybe_done {
  //! Definition of the [`MaybeDone`] combinator.

  use pin_project_lite::pin_project;
  use std::future::{Future, IntoFuture};
  use std::pin::Pin;
  use std::task::{ready, Context, Poll};

  pin_project! {
      /// A future that may have completed.
      #[derive(Debug)]
      #[project = MaybeDoneProj]
      #[project_replace = MaybeDoneProjReplace]
      #[repr(C)] // https://github.com/rust-lang/miri/issues/3780
      pub enum MaybeDone<Fut: Future> {
          /// A not-yet-completed future.
          Future { #[pin] future: Fut },
          /// The output of the completed future.
          Done { output: Fut::Output },
          /// The empty variant after the result of a [`MaybeDone`] has been
          /// taken using the [`take_output`](MaybeDone::take_output) method.
          Gone,
      }
  }

  /// Wraps a future into a `MaybeDone`.
  pub fn maybe_done<F: IntoFuture>(future: F) -> MaybeDone<F::IntoFuture> {
    MaybeDone::Future { future: future.into_future() }
  }

  impl<Fut: Future> MaybeDone<Fut> {
    /// Returns an [`Option`] containing a mutable reference to the output of the future.
    /// The output of this method will be [`Some`] if and only if the inner
    /// future has been completed and [`take_output`](MaybeDone::take_output)
    /// has not yet been called.
    pub fn output_mut(self: Pin<&mut Self>) -> Option<&mut Fut::Output> {
      match self.project() {
        MaybeDoneProj::Done { output } => Some(output),
        _ => None,
      }
    }

    /// Attempts to take the output of a `MaybeDone` without driving it
    /// towards completion.
    #[inline]
    pub fn take_output(self: Pin<&mut Self>) -> Option<Fut::Output> {
      match *self {
        MaybeDone::Done { .. } => {}
        MaybeDone::Future { .. } | MaybeDone::Gone => return None,
      };
      if let MaybeDoneProjReplace::Done { output } =
        self.project_replace(MaybeDone::Gone)
      {
        Some(output)
      } else {
        unreachable!()
      }
    }
  }

  impl<Fut: Future> Future for MaybeDone<Fut> {
    type Output = ();

    fn poll(
      mut self: Pin<&mut Self>,
      cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
      let output = match self.as_mut().project() {
        MaybeDoneProj::Future { future } => ready!(future.poll(cx)),
        MaybeDoneProj::Done { .. } => return Poll::Ready(()),
        MaybeDoneProj::Gone => panic!("MaybeDone polled after value taken"),
      };
      self.set(MaybeDone::Done { output });
      Poll::Ready(())
    }
  }

  // Test for https://github.com/tokio-rs/tokio/issues/6729
  #[cfg(test)]
  mod miri_tests {
    use super::maybe_done;

    use std::{
      future::Future,
      pin::Pin,
      sync::Arc,
      task::{Context, Poll, Wake},
    };

    struct ThingAdder<'a> {
      thing: &'a mut String,
    }

    impl Future for ThingAdder<'_> {
      type Output = ();

      fn poll(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
      ) -> Poll<Self::Output> {
        unsafe {
          *self.get_unchecked_mut().thing += ", world";
        }
        Poll::Pending
      }
    }

    #[test]
    fn maybe_done_miri() {
      let mut thing = "hello".to_owned();

      // The async block is necessary to trigger the miri failure.
      #[allow(clippy::redundant_async_block)]
      let fut = async move { ThingAdder { thing: &mut thing }.await };

      let mut fut = maybe_done(fut);
      let mut fut = unsafe { Pin::new_unchecked(&mut fut) };

      let waker = Arc::new(DummyWaker).into();
      let mut ctx = Context::from_waker(&waker);
      assert_eq!(fut.as_mut().poll(&mut ctx), Poll::Pending);
      assert_eq!(fut.as_mut().poll(&mut ctx), Poll::Pending);
    }

    struct DummyWaker;

    impl Wake for DummyWaker {
      fn wake(self: Arc<Self>) {}
    }
  }
}

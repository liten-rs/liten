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
        use $crate::utils::{maybe_done};
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
fn testing() {
  use std::future::ready;
  crate::runtime::Runtime::builder().block_on(async {
    let result = join!(ready(3u8), ready(""));
  })
}

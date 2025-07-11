use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

// used by liten::join!
/// A future that may have completed.
#[derive(Debug)]
pub enum MaybeDone<Fut: Future> {
  /// A not-yet-completed future
  Future(Fut),
  /// The output of the completed future
  Done(Fut::Output),
  /// The empty variant after the result of a [`MaybeDone`] has been
  /// taken using the [`take_output`](MaybeDone::take_output) method.
  Gone,
}

// Safe because we never generate `Pin<&mut Fut::Output>`
impl<Fut: Future + Unpin> Unpin for MaybeDone<Fut> {}

/// Wraps a future into a `MaybeDone`
pub fn maybe_done<Fut: Future>(future: Fut) -> MaybeDone<Fut> {
  MaybeDone::Future(future)
}

impl<Fut: Future> MaybeDone<Fut> {
  /// Returns an [`Option`] containing a mutable reference to the output of the future.
  /// The output of this method will be [`Some`] if and only if the inner
  /// future has been completed and [`take_output`](MaybeDone::take_output)
  /// has not yet been called.
  pub fn output_mut(self: Pin<&mut Self>) -> Option<&mut Fut::Output> {
    unsafe {
      let this = self.get_unchecked_mut();
      match this {
        MaybeDone::Done(res) => Some(res),
        _ => None,
      }
    }
  }

  /// Attempts to take the output of a `MaybeDone` without driving it
  /// towards completion.
  #[inline]
  pub fn take_output(self: Pin<&mut Self>) -> Option<Fut::Output> {
    unsafe {
      let this = self.get_unchecked_mut();
      match this {
        MaybeDone::Done(_) => {}
        MaybeDone::Future(_) | MaybeDone::Gone => return None,
      };
      if let MaybeDone::Done(output) = mem::replace(this, MaybeDone::Gone) {
        Some(output)
      } else {
        unreachable!()
      }
    }
  }
}

impl<Fut: Future> Future for MaybeDone<Fut> {
  type Output = ();

  fn poll(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Self::Output> {
    let res = unsafe {
      match self.as_mut().get_unchecked_mut() {
        MaybeDone::Future(a) => {
          std::task::ready!(Pin::new_unchecked(a).poll(cx))
        }
        MaybeDone::Done(_) => return Poll::Ready(()),
        MaybeDone::Gone => panic!("MaybeDone polled after value taken"),
      }
    };
    self.set(MaybeDone::Done(res));
    Poll::Ready(())
  }
}

#[derive(Debug)]
// Used as an associated type in a "sealed" trait.
#[allow(unreachable_pub)]
pub struct Ready<T>(Option<T>);

impl<T> Unpin for Ready<T> {}

impl<T> Future for Ready<T> {
  type Output = T;

  #[inline]
  fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
    Poll::Ready(self.0.take().unwrap())
  }
}

// /// Creates a future that is immediately ready with a success value.
//pub(crate) fn ok<T, E>(t: T) -> Ready<Result<T, E>> {
//    Ready(Some(Ok(t)))
//}

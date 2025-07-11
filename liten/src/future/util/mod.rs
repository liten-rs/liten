pub trait FnOnce1<A> {
  type Output;
  fn call_once(self, arg: A) -> Self::Output;
}

impl<T, A, R> FnOnce1<A> for T
where
  T: FnOnce(A) -> R,
{
  type Output = R;
  fn call_once(self, arg: A) -> R {
    self(arg)
  }
}

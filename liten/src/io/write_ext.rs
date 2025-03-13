mod flush;
mod write_all;

pub use flush::Flush;
pub use write_all::WriteAll;

use super::AsyncWrite;

pub trait AsyncWriteExt: AsyncWrite {
  /// Writes the entire contents of a buffer into this writer asynchronously.
  ///
  /// This is entirely equivalent to this:
  /// ```
  /// async fn write_all(&mut self, buf: &[u8]) -> io::Result<()>
  /// ```
  fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self> {
    WriteAll::new(self, buf)
  }

  /// Flushes the entire contents of a buffer for this writer asynchronously.
  ///
  /// This is entirely equivalent to this:
  /// ```
  /// async fn flush(&mut self) -> io::Result<()>
  /// ```
  fn flush(&mut self) -> Flush<'_, Self> {
    Flush::new(self)
  }
}

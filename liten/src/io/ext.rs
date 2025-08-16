use std::{
  future::Future,
  io,
  marker::PhantomData,
  pin::{pin, Pin},
  task::{Context, Poll},
};

use crate::io::{AsyncRead, AsyncWrite, BufResult};

impl<A: AsyncRead> AsyncReadExt for A {}
pub trait AsyncReadExt: AsyncRead {
  // TODO: Very inefishent with memory.
  fn read_all(
    &mut self,
    mut buf: Vec<u8>,
  ) -> impl Future<Output = BufResult<(), Vec<u8>>> {
    async {
      let total_len = buf.len();
      let mut filled = 0;

      while filled < total_len {
        let remaining = total_len - filled;
        let tmp = vec![0u8; remaining];
        let (result, read_buf) = self.read(tmp).await;

        match result {
          Ok(bytes_read) => {
            if bytes_read == 0 {
              return (
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "not enough bytes")),
                buf,
              );
            }

            buf[filled..filled + bytes_read]
              .copy_from_slice(&read_buf[..bytes_read]);
            filled += bytes_read;
          }
          Err(err) => {
            return (Err(err), buf);
          }
        }
      }

      (Ok(()), buf)
    }
  }

  fn read_u8(&mut self) -> impl Future<Output = io::Result<u8>>
  where
    Self: Sized,
  {
    ReadBytesFuture::<'_, Self, u8> { src: self, _t: PhantomData }
  }

  fn read_u16(&mut self) -> impl Future<Output = io::Result<u16>>
  where
    Self: Sized,
  {
    ReadBytesFuture::<'_, Self, u16> { src: self, _t: PhantomData }
  }

  fn read_u32(&mut self) -> impl Future<Output = io::Result<u32>>
  where
    Self: Sized,
  {
    ReadBytesFuture::<'_, Self, u32> { src: self, _t: PhantomData }
  }
  fn read_u64(&mut self) -> impl Future<Output = io::Result<u64>>
  where
    Self: Sized,
  {
    ReadBytesFuture::<'_, Self, u64> { src: self, _t: PhantomData }
  }

  fn read_u128(&mut self) -> impl Future<Output = io::Result<u128>>
  where
    Self: Sized,
  {
    ReadBytesFuture::<'_, Self, u128> { src: self, _t: PhantomData }
  }
}

pub struct ReadBytesFuture<'a, B: ?Sized, S> {
  src: &'a mut B,
  _t: PhantomData<S>,
}

macro_rules! impl_read_byte {
  ($ty:ty, $word_amount:expr) => {
    impl<'a, B> Future for ReadBytesFuture<'a, B, $ty>
    where
      B: AsyncRead,
    {
      type Output = io::Result<$ty>;
      fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
      ) -> Poll<Self::Output> {
        let buf = vec![0; $word_amount];
        match pin!(self.src.read_all(buf)).poll(cx) {
          Poll::Ready((maybe_error, buf)) => {
            maybe_error?;

            let array = <[u8; $word_amount]>::try_from(buf).expect("whut");

            Poll::Ready(Ok(<$ty>::from_be_bytes(array)))
          }
          Poll::Pending => Poll::Pending,
        }
      }
    }
  };
}

impl_read_byte!(u8, 1);
impl_read_byte!(u16, 2);
impl_read_byte!(u32, 4);
impl_read_byte!(u64, 8);
impl_read_byte!(u128, 16);

impl<A: AsyncWrite> AsyncWriteExt for A {}
pub trait AsyncWriteExt: AsyncWrite {
  // TODO: Very inefishent with memory.
  fn write_all(
    &mut self,
    buf: Vec<u8>,
  ) -> impl Future<Output = BufResult<(), Vec<u8>>> {
    // To remove lint warnings
    async {
      let total_len = buf.len();
      let mut written = 0;

      // Keep the original buffer to return it unchanged
      let original_buf = buf;

      while written < total_len {
        let remaining = total_len - written;
        let tmp = Vec::from(&original_buf[written..]);
        let (result, _tmp_buf) = self.write(tmp).await;

        match result {
          Err(err) => return (Err(err), original_buf),
          Ok(bytes_written) => {
            if bytes_written == 0 {
              return (
                Err(io::Error::new(io::ErrorKind::WriteZero, "failed to write the buffer")),
                original_buf,
              );
            }
            written += bytes_written;
          }
        }
      }

      (Ok(()), original_buf)
    }
  }

  fn write_u8(&mut self, num: u8) -> impl Future<Output = io::Result<()>>
  where
    Self: Sized,
  {
    WriteBytesFuture::<'_, Self, u8> { src: self, data: num }
  }

  fn write_u16(&mut self, num: u16) -> impl Future<Output = io::Result<()>>
  where
    Self: Sized,
  {
    WriteBytesFuture::<'_, Self, u16> { src: self, data: num }
  }

  fn write_u32(&mut self, num: u32) -> impl Future<Output = io::Result<()>>
  where
    Self: Sized,
  {
    WriteBytesFuture::<'_, Self, u32> { src: self, data: num }
  }
  fn write_u64(&mut self, num: u64) -> impl Future<Output = io::Result<()>>
  where
    Self: Sized,
  {
    WriteBytesFuture::<'_, Self, u64> { src: self, data: num }
  }

  fn write_u128(&mut self, num: u128) -> impl Future<Output = io::Result<()>>
  where
    Self: Sized,
  {
    WriteBytesFuture::<'_, Self, u128> { src: self, data: num }
  }
}

pub struct WriteBytesFuture<'a, B: ?Sized, S> {
  src: &'a mut B,
  data: S,
}

macro_rules! impl_write_byte {
  ($ty:ty, $word_amount:expr) => {
    impl<'a, B> Future for WriteBytesFuture<'a, B, $ty>
    where
      B: AsyncWrite,
    {
      type Output = io::Result<()>;
      fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
      ) -> Poll<Self::Output> {
        let buf = <$ty>::to_be_bytes(self.data);
        match pin!(self.src.write_all(std::vec::Vec::from(buf))).poll(cx) {
          Poll::Ready((maybe_error, _)) => Poll::Ready(maybe_error),
          Poll::Pending => Poll::Pending,
        }
      }
    }
  };
}

impl_write_byte!(u8, 1);
impl_write_byte!(u16, 2);
impl_write_byte!(u32, 4);
impl_write_byte!(u64, 8);
impl_write_byte!(u128, 16);

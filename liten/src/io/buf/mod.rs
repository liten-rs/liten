use crate::io::{AsyncRead, AsyncWrite, BufResult};

const DEFAULT_BUF_READER_CAPACITY: usize = 8 * 1024;

pub struct BufReader<R> {
  inner: R,
  buffer: Vec<u8>,
  position: usize,
  capacity: usize,
}

impl<R> BufReader<R> {
  pub fn new(inner: R) -> Self {
    Self::with_capacity(DEFAULT_BUF_READER_CAPACITY, inner)
  }

  pub fn with_capacity(capacity: usize, inner: R) -> Self {
    Self { inner, buffer: Vec::new(), position: 0, capacity: capacity.max(1) }
  }

  pub fn get_ref(&self) -> &R {
    &self.inner
  }

  pub fn get_mut(&mut self) -> &mut R {
    &mut self.inner
  }

  pub fn into_inner(self) -> R {
    self.inner
  }

  fn available(&self) -> usize {
    self.buffer.len().saturating_sub(self.position)
  }
}

impl<R: AsyncRead> AsyncRead for BufReader<R> {
  fn read(
    &mut self,
    mut out: Vec<u8>,
  ) -> impl core::future::Future<Output = BufResult<usize, Vec<u8>>> {
    async move {
      if out.is_empty() {
        return (Ok(0), out);
      }

      // Serve from internal buffer if available
      if self.available() > 0 {
        let available = self.available();
        let to_copy = out.len().min(available);
        out[..to_copy].copy_from_slice(
          &self.buffer[self.position..self.position + to_copy],
        );
        self.position += to_copy;
        return (Ok(to_copy), out);
      }

      // Otherwise, refill the internal buffer from the inner reader
      self.buffer.clear();
      self.position = 0;

      let (res, tmp) = self.inner.read(vec![0u8; self.capacity]).await;
      match res {
        Ok(n) => {
          if n == 0 {
            return (Ok(0), out);
          }
          let mut filled = tmp;
          if filled.len() != n {
            // If the inner returned a different length vec, ensure length matches bytes read
            if filled.len() > n {
              filled.truncate(n);
            } else {
              // Pad should never be needed, but keep invariant: buffer length == bytes read
              filled.resize(n, 0);
            }
          } else {
            filled.truncate(n);
          }
          self.buffer = filled;
          // Now copy into the output
          let to_copy = out.len().min(self.buffer.len());
          out[..to_copy].copy_from_slice(&self.buffer[..to_copy]);
          self.position = to_copy;
          (Ok(to_copy), out)
        }
        Err(err) => (Err(err), out),
      }
    }
  }
}

// ===== BufWriter =====

const DEFAULT_BUF_WRITER_CAPACITY: usize = 8 * 1024;

pub struct BufWriter<W> {
  inner: W,
  buffer: Option<Vec<u8>>, // None means empty buffer
  capacity: usize,
}

impl<W> BufWriter<W> {
  pub fn new(inner: W) -> Self {
    Self::with_capacity(DEFAULT_BUF_WRITER_CAPACITY, inner)
  }

  pub fn with_capacity(capacity: usize, inner: W) -> Self {
    Self { inner, buffer: None, capacity: capacity.max(1) }
  }

  pub fn get_ref(&self) -> &W {
    &self.inner
  }

  pub fn get_mut(&mut self) -> &mut W {
    &mut self.inner
  }

  pub fn into_inner(self) -> W {
    self.inner
  }

  fn buffered_len(&self) -> usize {
    self.buffer.as_ref().map(|b| b.len()).unwrap_or(0)
  }

  fn remaining(&self) -> usize {
    self.capacity.saturating_sub(self.buffered_len())
  }

  async fn flush_buf(&mut self) -> std::io::Result<()>
  where
    W: AsyncWrite,
  {
    loop {
      let buf = match self.buffer.take() {
        Some(b) => b,
        None => return Ok(()),
      };

      if buf.is_empty() {
        self.buffer = None;
        return Ok(());
      }

      let (res, mut returned) = self.inner.write(buf).await;
      match res {
        Ok(written) => {
          if written == 0 {
            // Put back buffer to preserve state on error
            self.buffer = Some(returned);
            return Err(std::io::Error::new(
              std::io::ErrorKind::WriteZero,
              "buffered write returned 0",
            ));
          }

          if written < returned.len() {
            // Keep the remaining bytes in buffer for next loop
            let _ = returned.drain(0..written);
            self.buffer = Some(returned);
            // Continue loop to flush remainder
            continue;
          } else {
            // Fully flushed this chunk; continue until None
            self.buffer = None;
            return Ok(());
          }
        }
        Err(err) => {
          // Put back what we tried to write
          self.buffer = Some(returned);
          return Err(err);
        }
      }
    }
  }
}

impl<W: AsyncWrite> AsyncWrite for BufWriter<W> {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl core::future::Future<Output = BufResult<usize, Vec<u8>>> {
    async move {
      if buf.is_empty() {
        return (Ok(0), buf);
      }

      // Case 1: fits in remaining buffer space
      if buf.len() <= self.remaining() {
        if self.buffer.is_none() {
          self.buffer = Some(Vec::new());
        }
        if let Some(b) = self.buffer.as_mut() {
          b.extend_from_slice(&buf);
        }
        // Auto-flush when buffer becomes exactly full
        if self.buffered_len() == self.capacity {
          if let Err(err) = self.flush_buf().await {
            return (Err(err), buf);
          }
        }
        return (Ok(buf.len()), buf);
      }

      // Case 2: buffer cannot fit entire input
      if buf.len() >= self.capacity {
        // Flush existing buffered data first to preserve write ordering
        if self.buffered_len() > 0 {
          if let Err(err) = self.flush_buf().await {
            return (Err(err), buf);
          }
        }
        // Large write goes directly to inner
        return self.inner.write(buf).await;
      }

      // Case 3: input smaller than capacity but doesn't fit -> flush then buffer it
      if let Err(err) = self.flush_buf().await {
        return (Err(err), buf);
      }
      if self.buffer.is_none() {
        self.buffer = Some(Vec::new());
      }
      if let Some(b) = self.buffer.as_mut() {
        b.extend_from_slice(&buf);
      }
      (Ok(buf.len()), buf)
    }
  }

  fn flush(&mut self) -> impl core::future::Future<Output = std::io::Result<()>> {
    async move {
      self.flush_buf().await?;
      self.inner.flush().await
    }
  }
}

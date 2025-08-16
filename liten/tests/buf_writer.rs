#![cfg(feature = "io")]

use liten::future::block_on;
use liten::io::BufWriter;
use liten::io::{AsyncWrite, BufResult};

#[derive(Default)]
struct MockWriter {
  pub written: Vec<u8>,
  pub max_per_call: usize,
  pub calls: usize,
}

impl MockWriter {
  fn new(max_per_call: usize) -> Self {
    Self { written: Vec::new(), max_per_call, calls: 0 }
  }
}

impl AsyncWrite for MockWriter {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl core::future::Future<Output = BufResult<usize, Vec<u8>>> {
    let allow = buf.len().min(self.max_per_call);
    self.written.extend_from_slice(&buf[..allow]);
    self.calls += 1;
    async move { (Ok(allow), buf) }
  }

  fn flush(
    &mut self,
  ) -> impl core::future::Future<Output = std::io::Result<()>> {
    async move { Ok(()) }
  }
}

#[liten::internal_test]
fn small_writes_buffer_until_flush() {
  let inner = MockWriter::new(usize::MAX);
  let mut writer = BufWriter::with_capacity(4, inner);

  // Write 3 bytes, should be buffered
  let (res1, input1) = block_on(writer.write(vec![1, 2, 3]));
  assert_eq!(res1.unwrap(), 3);
  assert_eq!(input1, vec![1, 2, 3]);
  assert_eq!(writer.get_ref().written, vec![]);

  // Write 1 more byte, buffer hits capacity, triggers flush
  let (res2, input2) = block_on(writer.write(vec![4]));
  assert_eq!(res2.unwrap(), 1);
  // After flush, inner should have [1,2,3,4]
  assert_eq!(writer.get_ref().written, vec![1, 2, 3, 4]);
  assert_eq!(input2, vec![4]);
}

#[liten::internal_test]
fn large_write_bypasses_buffer() {
  let inner = MockWriter::new(usize::MAX);
  let mut writer = BufWriter::with_capacity(4, inner);

  // Fill one byte in buffer
  let _ = block_on(writer.write(vec![9])).0.unwrap();
  // Now write a large vector; implementation flushes buffered data first, then writes payload directly
  let payload = vec![10, 11, 12, 13, 14, 15];
  let (res, returned) = block_on(writer.write(payload.clone()));
  assert_eq!(res.unwrap(), payload.len());
  assert_eq!(returned, payload);

  // Inner should have received [9] followed by [10..15]
  let mut want = vec![9];
  want.extend_from_slice(&payload);
  assert_eq!(writer.get_ref().written, want);

  // Buffer should be empty now; flushing should not change output
  block_on(writer.flush()).unwrap();
  assert_eq!(writer.get_ref().written, want);
}

#[liten::internal_test]
fn partial_inner_writes_are_handled() {
  let inner = MockWriter::new(3);
  let mut writer = BufWriter::with_capacity(4, inner);

  // This should be fully buffered (no flush yet)
  let _ = block_on(writer.write(vec![1, 2, 3, 4])).0.unwrap();
  // Trigger flush by writing one more byte, inner only accepts 3 per call
  let _ = block_on(writer.write(vec![5])).0.unwrap();

  // After the flush cycle, inner should have received first 4 bytes (via two partial writes)
  assert_eq!(writer.get_ref().written, vec![1, 2, 3, 4]);

  // The last byte should be in buffer, flush now
  block_on(writer.flush()).unwrap();
  assert_eq!(writer.get_ref().written, vec![1, 2, 3, 4, 5]);
}


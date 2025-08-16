#![cfg(feature = "io")]

use liten::future::block_on;
use liten::io::BufReader;
use liten::io::{AsyncRead, BufResult};

struct MockReader {
  data: Vec<u8>,
  chunk: usize,
  calls: usize,
}

impl MockReader {
  fn new(data: Vec<u8>, chunk: usize) -> Self {
    Self { data, chunk, calls: 0 }
  }

  fn calls(&self) -> usize {
    self.calls
  }
}

impl AsyncRead for MockReader {
  fn read(
    &mut self,
    buf: Vec<u8>,
  ) -> impl core::future::Future<Output = BufResult<usize, Vec<u8>>> {
    let allow = buf.len();
    let to_take = self.chunk.min(self.data.len()).min(allow);
    let out = self.data.drain(..to_take).collect::<Vec<u8>>();
    self.calls += 1;
    async move {
      let n = out.len();
      (Ok(n), out)
    }
  }
}

#[liten::internal_test]
fn buf_reader_serves_from_buffer_without_extra_reads() {
  let inner = MockReader::new(vec![1, 2, 3, 4, 5, 6], 6);
  let mut reader = BufReader::with_capacity(4, inner);

  // First read triggers one inner read, fills internal buffer of 4
  let (r1, out1) = block_on(reader.read(vec![0u8; 2]));
  assert_eq!(r1.unwrap(), 2);
  assert_eq!(out1, vec![1, 2]);
  assert_eq!(reader.get_ref().calls(), 1);

  // Second read should be served entirely from buffer
  let (r2, out2) = block_on(reader.read(vec![0u8; 2]));
  assert_eq!(r2.unwrap(), 2);
  assert_eq!(out2, vec![3, 4]);
  assert_eq!(reader.get_ref().calls(), 1);

  // Third read requires a refill
  let (r3, out3) = block_on(reader.read(vec![0u8; 2]));
  assert_eq!(r3.unwrap(), 2);
  assert_eq!(out3, vec![5, 6]);
  assert_eq!(reader.get_ref().calls(), 2);
}

#[liten::internal_test]
fn buf_reader_zero_len_read_is_noop() {
  let inner = MockReader::new(vec![10, 11, 12], 3);
  let mut reader = BufReader::with_capacity(4, inner);

  let (r, out) = block_on(reader.read(Vec::new()));
  assert_eq!(r.unwrap(), 0);
  assert!(out.is_empty());
  // No inner read should have occurred for zero-length
  assert_eq!(reader.get_ref().calls(), 0);
}

#[liten::internal_test]
fn buf_reader_eof_returns_zero() {
  let inner = MockReader::new(vec![42], 1);
  let mut reader = BufReader::with_capacity(4, inner);

  // Consume the one byte
  let (r1, out1) = block_on(reader.read(vec![0u8; 8]));
  assert_eq!(r1.unwrap(), 1);
  assert_eq!(out1[0], 42);

  // EOF now
  let (r2, out2) = block_on(reader.read(vec![0u8; 8]));
  assert_eq!(r2.unwrap(), 0);
  assert_eq!(out2.len(), 8);
}


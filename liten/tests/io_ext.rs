#![cfg(feature = "io")]

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

use liten::future::block_on;
use liten::io::{
  AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufResult,
};

struct MockReader {
  data: Vec<u8>,
  chunk: usize,
}

impl MockReader {
  fn new(data: Vec<u8>, chunk: usize) -> Self {
    Self { data, chunk }
  }
}

impl AsyncRead for MockReader {
  fn read(
    &mut self,
    _buf: Vec<u8>,
  ) -> impl std::future::Future<Output = BufResult<usize, Vec<u8>>> {
    let take = self.chunk.min(self.data.len());
    let out = self.data.drain(..take).collect::<Vec<u8>>();
    async move {
      let len = out.len();
      (Ok(len), out)
    }
  }
}

struct MockWriter {
  written: Vec<u8>,
  max_per_call: usize,
  calls: AtomicUsize,
}

impl MockWriter {
  fn new(max_per_call: usize) -> Self {
    Self { written: Vec::new(), max_per_call, calls: AtomicUsize::new(0) }
  }
}

impl AsyncWrite for MockWriter {
  fn write(
    &mut self,
    buf: Vec<u8>,
  ) -> impl std::future::Future<Output = BufResult<usize, Vec<u8>>> {
    let allowed = buf.len().min(self.max_per_call);
    self.written.extend_from_slice(&buf[..allowed]);
    self.calls.fetch_add(1, Ordering::SeqCst);
    async move { (Ok(allowed), buf) }
  }
  fn flush(&mut self) -> impl std::future::Future<Output = io::Result<()>> {
    async move { Ok(()) }
  }
}

#[liten::internal_test]
fn read_all_exact_success() {
  let input = vec![1, 2, 3, 4, 5, 6, 7, 8];
  let mut reader = MockReader::new(input.clone(), 3);
  let buf = vec![0u8; input.len()];
  let (res, out) = block_on(reader.read_all(buf));
  assert!(res.is_ok());
  assert_eq!(out, input);
}

#[liten::internal_test]
fn read_all_unexpected_eof() {
  let input = vec![1, 2, 3, 4];
  let mut reader = MockReader::new(input.clone(), 10);
  // Request more bytes than available
  let buf = vec![0u8; input.len() + 2];
  let (res, _out) = block_on(reader.read_all(buf));
  assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::UnexpectedEof));
}

#[liten::internal_test]
fn write_all_partial_writes() {
  let mut writer = MockWriter::new(3);
  let payload = vec![10, 11, 12, 13, 14, 15, 16];
  let (res, returned) = block_on(writer.write_all(payload.clone()));
  assert!(res.is_ok());
  assert_eq!(returned, payload);
  assert_eq!(writer.written, payload);
  assert!(writer.calls.load(Ordering::SeqCst) >= 3);
}

#[liten::internal_test]
fn write_all_write_zero_error() {
  struct ZeroWriter;
  impl AsyncWrite for ZeroWriter {
    fn write(
      &mut self,
      buf: Vec<u8>,
    ) -> impl std::future::Future<Output = BufResult<usize, Vec<u8>>> {
      async move { (Ok(0), buf) }
    }
    fn flush(&mut self) -> impl std::future::Future<Output = io::Result<()>> {
      async move { Ok(()) }
    }
  }
  let mut writer = ZeroWriter;
  let payload = vec![1, 2, 3, 4];
  let (res, _returned) = block_on(writer.write_all(payload));
  assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::WriteZero));
}

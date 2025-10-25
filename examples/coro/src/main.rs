use std::{ffi::CString, os::fd::RawFd};

#[tokio::main(flavor = "current_thread")]
async fn main() {
  let fd: RawFd = lio::openat(
    libc::AT_FDCWD,
    CString::new("README.md").unwrap(),
    libc::O_RDONLY,
  )
  .await
  .unwrap();

  let mut buf = vec![0u8; 100];
  buf.fill(0);

  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );

  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );

  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );

  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;

  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );
  let (res, buf) = lio::read(fd, buf, -1).await;
  println!(
    "bytes: {}, buf: {:?}",
    res.unwrap(),
    String::from_utf8(buf.clone())
  );

  lio::close(fd).await.unwrap();
}

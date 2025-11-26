#![cfg(linux)]

use std::time::Instant;

#[test]
fn test_close_timing() {
  let outer_start = Instant::now();
  liten::block_on(async {
    let inner_start = Instant::now();
    let mut pipe1_fds = [0i32; 2];
    let mut pipe2_fds = [0i32; 2];
    unsafe {
      assert_eq!(libc::pipe(pipe1_fds.as_mut_ptr()), 0);
      assert_eq!(libc::pipe(pipe2_fds.as_mut_ptr()), 0);
    }

    // Write some data
    let test_data = b"Hello!";
    unsafe {
      libc::write(
        pipe1_fds[1],
        test_data.as_ptr() as *const libc::c_void,
        test_data.len(),
      );
    }

    // Do a tee operation
    let tee_start = Instant::now();
    lio::tee(pipe1_fds[0], pipe2_fds[1], test_data.len() as u32)
      .await
      .unwrap();
    println!("Tee took: {:?}", tee_start.elapsed());

    lio::close(pipe1_fds[0]).await.unwrap();
    lio::close(pipe1_fds[1]).await.unwrap();
    lio::close(pipe2_fds[0]).await.unwrap();
    lio::close(pipe2_fds[1]).await.unwrap();

    println!("Inner async block took: {:?}", inner_start.elapsed());
  });
  println!("Total block_on took: {:?}", outer_start.elapsed());
}

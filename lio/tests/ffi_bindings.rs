/// Test that verifies FFI bindings can be compiled and linked with C and C++
use std::path::PathBuf;
use std::process::Command;

const C_TEST_SOURCE: &str = r#"
#include <sys/socket.h>
#include <lio.h>

void test_callback(int32_t result) {
    (void)result;
}

int main(void) {
    lio_close(999, test_callback);
    return 0;
}
"#;

const CPP_TEST_SOURCE: &str = r#"
#include <sys/socket.h>
#include <lio.h>

void test_callback(int32_t result) {
    (void)result;
}

int main() {
    lio_close(999, test_callback);
    return 0;
}
"#;

const C_RUN_SOURCE: &str = r#"
#include <stdio.h>
#include <sys/socket.h>
#include <lio.h>

void test_callback(int32_t result) {
    printf("Callback received: %d\n", result);
}

int main(void) {
    lio_close(999, test_callback);
    return 0;
}
"#;

fn project_root() -> PathBuf {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  dbg!(PathBuf::from(path.parent().unwrap()))
}

fn target_dir() -> PathBuf {
  project_root().join("target/release")
}

#[cfg(target_os = "macos")]
const EXT: &str = "dylib";

#[cfg(target_os = "linux")]
const EXT: &str = "so";

#[cfg(target_os = "windows")]
const EXT: &str = "dll";

#[test]
fn test_ffi_bindings() {
  let root = project_root();

  // Build the library with FFI feature and generate bindings using make
  let output = Command::new("make")
    .current_dir(&root)
    .args(&["lio-cbuild"])
    .output()
    .expect("Failed to run make lio-cbuild");

  assert!(
    output.status.success(),
    "make lio-cbuild failed:\n{}",
    String::from_utf8_lossy(&output.stderr)
  );

  if let Ok(value) = String::from_utf8(output.stdout) {
    println!("value: {value}");
  }

  // Verify lio.h exists
  assert!(root.join("lio/include/lio.h").exists(), "lio.h was not generated");

  // Verify library exists
  let lib_path = target_dir().join(&format!("liblio.{EXT}"));
  assert!(
    lib_path.exists(),
    "liblio.{} was not built: Couldn't find path {}",
    EXT,
    lib_path
  );
  dbg!(std::fs::read_dir(target_dir()).unwrap().collect::<Vec<_>>());

  // Create a simple C test file
  let c_source = target_dir().join("test_ffi_compile.c");
  std::fs::create_dir_all(target_dir()).ok();

  std::fs::write(&c_source, C_TEST_SOURCE)
    .expect("Failed to write C test file");

  // Compile the C file
  let output = Command::new("gcc")
    .args(&[
      "-c",
      c_source.to_str().unwrap(),
      "-o",
      target_dir().join("test_ffi_compile.o").to_str().unwrap(),
      "-Ltarget/release",
      "-I",
      root.join("lio/include").to_str().unwrap(),
    ])
    .output()
    .expect("Failed to compile C test");

  assert!(
    output.status.success(),
    "C compilation failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  // Create a simple C++ test file
  let cpp_source = target_dir().join("test_ffi_compile.cpp");
  std::fs::write(&cpp_source, CPP_TEST_SOURCE)
    .expect("Failed to write C++ test file");

  // Compile the C++ file
  let output = Command::new("c++")
    .args(&[
      "-c",
      cpp_source.to_str().unwrap(),
      "-o",
      target_dir().join("test_ffi_compile_cpp.o").to_str().unwrap(),
      &format!("-L{}", target_dir().display()),
      "-I",
      root.join("lio/include").to_str().unwrap(),
    ])
    .output()
    .expect("Failed to compile C++ test");

  assert!(
    output.status.success(),
    "C++ compilation failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  // Create C test program for linking and running
  let c_run_source = target_dir().join("test_ffi_run.c");
  std::fs::write(&c_run_source, C_RUN_SOURCE)
    .expect("Failed to write C test file");

  let exe_path = target_dir().join("test_ffi_run");

  // Compile and link
  let output = Command::new("cc")
    .args(&[
      c_run_source.to_str().unwrap(),
      "-L",
      target_dir().to_str().unwrap(),
      "-llio",
      "-o",
      exe_path.to_str().unwrap(),
      "-I",
      root.join("lio/include").to_str().unwrap(),
    ])
    .output()
    .expect("Failed to compile C test");

  assert!(
    output.status.success(),
    "C linking failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  // Run the test
  let output = Command::new(&exe_path)
    .env("DYLD_LIBRARY_PATH", target_dir().join("debug"))
    .output()
    .expect("Failed to run C test");

  assert!(
    output.status.success(),
    "C test execution failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

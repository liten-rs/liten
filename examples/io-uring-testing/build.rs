fn main() {
  cc::Build::new()
    .file("./liburing/src/ffi.c")
    .include("./liburing/src/include")
    .compile("liburing");
}

fn main() {
  cfg_aliases::cfg_aliases! {
      linux: { target_os = "linux" },
      macos: { target_os = "macos" },
  }
}

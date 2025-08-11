fn main() {
  cfg_aliases::cfg_aliases! {
      linux: { target_os = "linux" },
      not_linux: { not(target_os = "linux") }
  }
}

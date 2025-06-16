pub const fn has_flag(byte: u8, flag: u8) -> bool {
  (byte & flag) != 0
}

#[test]
fn test_has_flag() {
  assert!(has_flag(0b0000_0001, 0b0000_0001));
  assert!(!has_flag(0b0000_0001, 0b0000_0010));
}

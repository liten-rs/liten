pub fn breakdown_milliseconds(
  total_ms: usize,
) -> (usize, usize, usize, usize, usize) {
  let milliseconds = total_ms % 1000;
  let seconds = (total_ms / 1000) % 60;
  let minutes = (total_ms / (1000 * 60)) % 60;
  let hours = (total_ms / (1000 * 60 * 60)) % 24;
  let days = total_ms / (1000 * 60 * 60 * 24);

  (days, hours, minutes, seconds, milliseconds)
}

#[test]
fn test_zero() {
  assert_eq!(breakdown_milliseconds(0), (0, 0, 0, 0, 0));
}

#[test]
fn test_just_milliseconds() {
  assert_eq!(breakdown_milliseconds(999), (0, 0, 0, 0, 999));
  assert_eq!(breakdown_milliseconds(1), (0, 0, 0, 0, 1));
}

#[test]
fn test_just_seconds() {
  assert_eq!(breakdown_milliseconds(1_000), (0, 0, 0, 1, 0));
  assert_eq!(breakdown_milliseconds(59_999), (0, 0, 0, 59, 999));
}

#[test]
fn test_just_minutes() {
  assert_eq!(breakdown_milliseconds(60_000), (0, 0, 1, 0, 0));
  assert_eq!(breakdown_milliseconds(3_599_999), (0, 0, 59, 59, 999));
}

#[test]
fn test_just_hours() {
  assert_eq!(breakdown_milliseconds(3_600_000), (0, 1, 0, 0, 0));
  assert_eq!(breakdown_milliseconds(86_399_999), (0, 23, 59, 59, 999));
}

#[test]
fn test_just_days() {
  assert_eq!(breakdown_milliseconds(86_400_000), (1, 0, 0, 0, 0));
  assert_eq!(breakdown_milliseconds(172_799_999), (1, 23, 59, 59, 999));
}

#[test]
fn test_multiple_days() {
  assert_eq!(breakdown_milliseconds(2 * 86_400_000), (2, 0, 0, 0, 0));
  assert_eq!(
    breakdown_milliseconds(10 * 86_400_000 + 3_600_000 + 60_000 + 1000 + 1),
    (10, 1, 1, 1, 1)
  );
}

#[test]
fn test_mixed_values() {
  // 1 day, 2 hours, 3 minutes, 4 seconds, 5 milliseconds
  #[allow(clippy::identity_op)]
  let total = 1 * 86_400_000 + 2 * 3_600_000 + 3 * 60_000 + 4 * 1000 + 5;
  assert_eq!(breakdown_milliseconds(total), (1, 2, 3, 4, 5));
}

#[test]
fn test_large_values() {
  // 123 days, 22 hours, 33 minutes, 44 seconds, 555 milliseconds
  let total = 123 * 86_400_000 + 22 * 3_600_000 + 33 * 60_000 + 44 * 1000 + 555;
  assert_eq!(breakdown_milliseconds(total), (123, 22, 33, 44, 555));
}

use std::time::Instant;

use crate::time::wheel::TimerTickResult;

use super::wheel::Wheel;

#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub struct TimerId(usize);

impl TimerId {
  pub(in crate::time) fn new(num: usize) -> Self {
    Self(num)
  }
}

const W0_BITS: usize = 8;
const W1_BITS: usize = 6;
const W2_BITS: usize = 6;
const W3_BITS: usize = 6;

const W0_SIZE: usize = 1 << W0_BITS;
const W1_SIZE: usize = 1 << W1_BITS;
const W2_SIZE: usize = 1 << W2_BITS;
const W3_SIZE: usize = 1 << W3_BITS;

const W0_MASK: usize = W0_SIZE - 1;
const W1_MASK: usize = W1_SIZE - 1;
const W2_MASK: usize = W2_SIZE - 1;
const W3_MASK: usize = W3_SIZE - 1;

// Wrapper to track timer entry with its duration breakdown for cascading
pub struct TimerEntry<T> {
  value: T,
  when_done_timestamp: usize,
}

impl<T> TimerEntry<T> {
  fn into_value(self) -> T {
    self.value
  }
}

pub enum ClockMode {
  WallClock { when_init: Instant },
  Logical { current_time: usize },
}

impl ClockMode {
  pub fn logical() -> Self {
    ClockMode::Logical { current_time: 0 }
  }

  pub fn wall(instant: Instant) -> Self {
    ClockMode::WallClock { when_init: instant }
  }
}

pub(in crate::time) struct Clock<T> {
  // 256 slots * 1ms = 256ms
  w0: Wheel<W0_SIZE, TimerEntry<T>>,
  // 64 slots * 256ms = 16.4s
  w1: Wheel<W1_SIZE, TimerEntry<T>>,
  // 64 slots * 16.4s = 17.5min
  w2: Wheel<W2_SIZE, TimerEntry<T>>,
  // 64 slots * 16.4s = 18.6hr
  w3: Wheel<W3_SIZE, TimerEntry<T>>,
  mode: ClockMode,
}

impl<T> Clock<T> {
  pub fn new() -> Self {
    Self::new_with_mode(ClockMode::wall(Instant::now()))
  }

  pub fn new_with_mode(mode: ClockMode) -> Self {
    Self {
      w0: Wheel::new_with_position(0),
      w1: Wheel::new_with_position(0),
      w2: Wheel::new_with_position(0),
      w3: Wheel::new_with_position(0),
      mode,
    }
  }

  fn current_time(&self) -> usize {
    match &self.mode {
      ClockMode::WallClock { when_init } => {
        when_init.elapsed().as_millis().try_into().unwrap()
      }
      ClockMode::Logical { current_time } => *current_time,
    }
  }

  fn insert_time_entry(&mut self, entry: TimerEntry<T>) {
    let duration = entry.when_done_timestamp - self.current_time();
    // Determine which wheel to use based on the duration
    // Use bit decomposition to find the highest significant wheel
    let w0_ticks = duration & W0_MASK;
    let w1_ticks = (duration >> W0_BITS) & W1_MASK;
    let w2_ticks = (duration >> (W0_BITS + W1_BITS)) & W2_MASK;
    let w3_ticks = (duration >> (W0_BITS + W1_BITS + W2_BITS)) & W3_MASK;

    // Insert into the highest non-zero wheel, or w0 if all are 0
    if w3_ticks > 0 {
      self.w3.insert(w3_ticks, entry);
    } else if w2_ticks > 0 {
      self.w2.insert(w2_ticks, entry);
    } else if w1_ticks > 0 {
      self.w1.insert(w1_ticks, entry);
    } else if w0_ticks > 0 {
      self.w0.insert(w0_ticks, entry);
    }
  }

  /// Insert a timer that will expire after `duration` units
  pub fn insert(&mut self, value: T, duration: usize) {
    let entry =
      TimerEntry { value, when_done_timestamp: self.current_time() + duration };
    self.insert_time_entry(entry);
  }

  pub fn advance(&mut self, units: usize) -> Vec<T> {
    // Update logical time if in Logical mode
    if let ClockMode::Logical { current_time } = &mut self.mode {
      *current_time += units;
    };
    let current_time = self.current_time();

    let mut all_slots = Vec::new();

    let TimerTickResult { slots, resetted_counter } = self.w0.advance(units);

    all_slots.extend(
      slots.into_iter().map(TimerEntry::into_value).collect::<Vec<T>>(),
    );

    let TimerTickResult { slots, resetted_counter } =
      self.w1.advance(resetted_counter);

    for slot in slots {
      if slot.when_done_timestamp > current_time {
        self.insert_time_entry(slot);
      } else {
        all_slots.push(slot.into_value());
      }
    }

    let TimerTickResult { slots, resetted_counter } =
      self.w2.advance(resetted_counter);

    for slot in slots {
      if slot.when_done_timestamp > current_time {
        self.insert_time_entry(slot);
      } else {
        all_slots.push(slot.into_value());
      }
    }

    let TimerTickResult { slots, resetted_counter: _ } =
      self.w3.advance(resetted_counter);

    for slot in slots {
      if slot.when_done_timestamp > current_time {
        self.insert_time_entry(slot);
      } else {
        all_slots.push(slot.into_value());
      }
    }

    all_slots
  }

  pub fn peek(&self) -> Option<usize> {
    self
      .w0
      .peak_nearest()
      .or(self.w1.peak_nearest().map(|value| value * W0_SIZE))
      .or(self.w2.peak_nearest().map(|value| value * W0_SIZE * W1_SIZE))
      .or(
        self.w3.peak_nearest().map(|value| value * W0_SIZE * W1_SIZE * W2_SIZE),
      )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Helper function for tests to create TimerEntry
  fn entry<T>(value: T, when_done_timestamp: usize) -> TimerEntry<T> {
    TimerEntry { value, when_done_timestamp }
  }

  // Helper to create a logical clock for testing
  fn logical_clock<T>() -> Clock<T> {
    Clock::new_with_mode(ClockMode::Logical { current_time: 0 })
  }

  #[test]
  fn test_clock_advance_zero_units() {
    let mut clock: Clock<i32> = logical_clock();
    let result = clock.advance(0);
    assert_eq!(result.len(), 0, "Advancing 0 units should return no items");
  }

  // #[test]
  // fn test_clock_advance_w0_only() {
  //   let mut clock: Clock<i32> = logical_clock();
  //   // Insert items in w0 range (0-255)
  //   clock.w0.insert(100, 1_000_000);
  //   clock.w0.insert(200, 1_000_000);
  //
  //   let result = clock.advance(10);
  //
  //   assert_eq!(result.len(), 2, "Should return both items from w0");
  //   assert!(result.contains(&100), "Should contain first item");
  //   assert!(result.contains(&200), "Should contain second item");
  // }

  // #[test]
  // fn test_clock_advance_w0_boundary() {
  //   let mut clock: Clock<i32> = logical_clock();
  //   // Test advancing exactly W0_SIZE (256) units
  //   clock.w0.insert(42, 1_000_000);
  //
  //   let result = clock.advance(100);
  //
  //   assert!(result.contains(&42), "Should retrieve item at boundary");
  // }

  // #[test]
  // fn test_clock_advance_multiple_wheels() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert items in different wheels
  //   // Timestamps must be <= the time we advance to for them to fire
  //   let w1_units = (5 << W0_BITS) | 5; // = 261
  //   clock.w0.insert(1, 5); // w0 item - fires at 5
  //   clock.w1.insert(2, 256 + 5); // w1 item - fires at 261
  //   clock.w2.insert(3, 100); // w2 item - fires early
  //   clock.w3.insert(4, 50); // w3 item - fires early
  //
  //   // Advance to trigger w1 (units = 256 + 5 = 261)
  //   let result = clock.advance(w1_units);
  //
  //   // Should collect items from multiple wheels
  //   assert!(result.len() >= 2, "Should return items from multiple wheels");
  // }
  //
  // #[test]
  // fn test_clock_advance_with_carry_over() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert item that should be reached with carry over
  //   clock.w0.insert(W0_SIZE - 1, 99, 1_000_000);
  //
  //   // Advance enough to wrap w0 and trigger carry
  //   let result = clock.advance(1_000);
  //   dbg!(&result);
  //
  //   assert!(result.contains(&99), "Should handle carry over from w0 wrap");
  // }
  //
  // #[test]
  // fn test_clock_advance_empty_clock() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   let result = clock.advance(1_000);
  //
  //   assert_eq!(result.len(), 0, "Empty clock should return no items");
  // }
  //
  // #[test]
  // fn test_clock_advance_large_units() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert items in various wheels
  //   clock.w0.insert(10, 1, 1_000_000);
  //   clock.w1.insert(10, 2, 1_000_000);
  //   clock.w2.insert(10, 3, 1_000_000);
  //
  //   // Advance a large number of units that spans multiple wheels
  //   let large_units = (10 << (W0_BITS + W1_BITS)) | (10 << W0_BITS) | 10;
  //   let result = clock.advance(large_units);
  //
  //   // Should collect items from all wheels that were advanced
  //   assert!(result.len() >= 1, "Should return items from advanced wheels");
  // }
  //
  // #[test]
  // fn test_clock_advance_bit_decomposition() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Test that bit decomposition works correctly
  //   // units = 0b00000110_00000101_00000100_00000011
  //   //         w3      w2      w1      w0
  //   let units = (6 << (W0_BITS + W1_BITS + W2_BITS))
  //     | (5 << (W0_BITS + W1_BITS))
  //     | (4 << W0_BITS)
  //     | 3;
  //
  //   // Insert items to verify correct wheel advancement
  //   clock.w0.insert(1, 10, 1_000_000);
  //
  //   let result = clock.advance(units);
  //
  //   // The decomposition should properly separate into w0, w1, w2, w3 components
  //   assert!(result.contains(&10), "Should find item after complex advancement");
  // }
  //
  // #[test]
  // fn test_clock_advance_sequential_advances() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert items at different positions
  //   clock.w0.insert(5, 100, 1_000_000);
  //   clock.w0.insert(15, 200, 1_000_000);
  //   clock.w0.insert(25, 300, 1_000_000);
  //
  //   // Advance in steps
  //   let result1 = clock.advance(5);
  //   assert_eq!(result1.len(), 1, "First advance should return 1 item");
  //   assert!(result1.contains(&100));
  //
  //   let result2 = clock.advance(10);
  //   assert_eq!(result2.len(), 1, "Second advance should return 1 item");
  //   assert!(result2.contains(&200));
  //
  //   let result3 = clock.advance(10);
  //   assert_eq!(result3.len(), 1, "Third advance should return 1 item");
  //   assert!(result3.contains(&300));
  // }
  //
  // #[test]
  // fn test_clock_advance_all_wheels_boundary() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Test the maximum value that fits in all wheels
  //   let max_w0 = W0_MASK;
  //   let max_w1 = W1_MASK << W0_BITS;
  //   let max_w2 = W2_MASK << (W0_BITS + W1_BITS);
  //   let max_w3 = W3_MASK << (W0_BITS + W1_BITS + W2_BITS);
  //
  //   let max_units = max_w3 | max_w2 | max_w1 | max_w0;
  //
  //   clock.w0.insert(1, 42, 1_000_000);
  //
  //   // Should not panic with maximum units
  //   let result = clock.advance(max_units);
  //
  //   assert!(result.len() >= 0, "Should handle maximum units without panic");
  // }
  //
  // #[test]
  // fn test_clock_advance_w0_wrap_multiple_times() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert item that should be collected after multiple w0 wraps
  //   clock.w0.insert(50, 999, 1_000_000);
  //
  //   // Advance enough to wrap w0 multiple times
  //   let units = W0_SIZE * 3 + 50;
  //   let result = clock.advance(units);
  //
  //   assert!(result.contains(&999), "Should handle multiple w0 wraps");
  // }
  //
  // #[test]
  // fn test_clock_advance_preserves_order() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert multiple items in the same slot
  //   clock.w0.insert(5, 1, 1_000_000);
  //   clock.w0.insert(5, 2, 1_000_000);
  //   clock.w0.insert(5, 3, 1_000_000);
  //
  //   let result = clock.advance(5);
  //
  //   assert_eq!(result.len(), 3, "Should return all items from same slot");
  //   assert!(result.contains(&1));
  //   assert!(result.contains(&2));
  //   assert!(result.contains(&3));
  // }
  //
  // #[test]
  // fn test_clock_advance_with_string_type() {
  //   let mut clock: Clock<String> = logical_clock();
  //
  //   clock.w0.insert(10, "first".to_string(), 1_000_000);
  //   clock.w0.insert(20, "second".to_string(), 1_000_000);
  //
  //   let result = clock.advance(20);
  //
  //   assert_eq!(result.len(), 2);
  //   assert!(result.contains(&"first".to_string()));
  //   assert!(result.contains(&"second".to_string()));
  // }
  //
  // #[test]
  // fn test_clock_advance_single_unit() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   clock.w0.insert(1, 42, 1_000_000);
  //
  //   let result = clock.advance(1);
  //
  //   assert_eq!(result.len(), 1, "Single unit advance should work");
  //   assert_eq!(result[0], 42);
  // }
  //
  // #[test]
  // fn test_clock_advance_w1_transition() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert in w1 range
  //   clock.w1.insert(1, 100, 1_000_000);
  //
  //   // Advance exactly to w1 boundary (256 units = 1 << W0_BITS)
  //   let w1_unit = 1 << W0_BITS;
  //   let result = clock.advance(w1_unit);
  //
  //   // This tests the transition from w0 to w1
  //   assert!(result.len() >= 0, "Should handle w1 transition");
  // }
  //
  // #[test]
  // fn test_clock_advance_w2_transition() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert in w2 range
  //   clock.w2.insert(1, 200, 1_000_000);
  //
  //   // Advance to w2 boundary
  //   let w2_unit = 1 << (W0_BITS + W1_BITS);
  //   let result = clock.advance(w2_unit);
  //
  //   assert!(result.len() >= 0, "Should handle w2 transition");
  // }
  //
  // #[test]
  // fn test_clock_advance_w3_transition() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert in w3 range
  //   clock.w3.insert(1, 300, 1_000_000);
  //
  //   // Advance to w3 boundary
  //   let w3_unit = 1 << (W0_BITS + W1_BITS + W2_BITS);
  //   let result = clock.advance(w3_unit);
  //
  //   assert!(result.len() >= 0, "Should handle w3 transition");
  // }
  //
  // #[test]
  // fn test_clock_advance_mixed_wheels_collection() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Insert items across all wheels
  //   clock.w0.insert(10, 1_000_000);
  //   clock.w0.insert(20, 1_000_000);
  //   clock.w1.insert(30, 1_000_000);
  //   clock.w2.insert(40, 1_000_000);
  //
  //   // Advance with complex pattern
  //   let units = (1 << (W0_BITS + W1_BITS)) | (2 << W0_BITS) | 15;
  //   let result = clock.advance(units);
  //
  //   // Should collect items from multiple wheels
  //   assert!(result.len() >= 2, "Should collect from multiple wheels");
  // }
  //
  // #[test]
  // fn test_clock_new_initializes_correctly() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   // Verify that advancing on a new clock returns nothing
  //   let result = clock.advance(100);
  //
  //   assert_eq!(result.len(), 0, "New clock should be empty");
  // }
  //
  // #[test]
  // fn test_clock_advance_result_is_vec() {
  //   let mut clock: Clock<i32> = logical_clock();
  //
  //   clock.w0.insert(42, 1_000_000);
  //
  //   let result = clock.advance(5);
  //
  //   // Verify it returns a Vec that can be manipulated
  //   assert_eq!(result.len(), 1);
  //   assert!(result.into_iter().any(|x| x == 42));
  // }

  // Tests for Clock::insert method
  #[test]
  fn test_clock_insert_w0_range() {
    let mut clock: Clock<i32> = logical_clock();

    // Test various durations in w0 range (1-255)
    // Note: duration 0 goes to current slot and won't fire
    clock.insert(1, 1);
    clock.insert(2, 50);
    clock.insert(3, 100);
    clock.insert(4, 255);

    // Advance to collect all items
    let result = clock.advance(255);

    assert_eq!(result.len(), 4, "Should insert 4 items into w0");
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
    assert!(result.contains(&4));
  }

  #[test]
  fn test_clock_insert_w1_range() {
    let mut clock: Clock<i32> = logical_clock();

    // Test durations in w1 range (256-16383)
    // 256 = 1 << 8 (first w1 slot)
    clock.insert(100, 256);
    clock.insert(200, 512);
    clock.insert(300, 1_024);

    // Advance to trigger w1
    let result = clock.advance(1_024);

    assert_eq!(result.len(), 3, "Should collect all w1 items");
    assert!(result.contains(&100));
    assert!(result.contains(&200));
    assert!(result.contains(&300));
  }

  #[test]
  fn test_clock_insert_w2_range() {
    let mut clock: Clock<i32> = logical_clock();

    // Test durations in w2 range (16384+)
    // 16384 = 1 << 14 (first w2 slot)
    clock.insert(1_000, 16_384);
    clock.insert(2_000, 32_768);

    // Advance to trigger w2
    let result = clock.advance(32768);

    assert_eq!(result.len(), 2, "Should collect all w2 items");
    assert!(result.contains(&1_000));
    assert!(result.contains(&2_000));
  }

  #[test]
  fn test_clock_insert_w3_range() {
    let mut clock: Clock<i32> = logical_clock();

    // Test durations in w3 range (1048576+)
    // 1048576 = 1 << 20 (first w3 slot)
    clock.insert(10_000, 1_048_576);
    clock.insert(20_000, 209_7152);

    // Advance to trigger w3
    let result = clock.advance(2_097_152);

    assert_eq!(result.len(), 2, "Should collect all w3 items");
    assert!(result.contains(&10_000));
    assert!(result.contains(&20_000));
  }

  #[test]
  fn test_clock_insert_mixed_wheels() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert items that should go into different wheels
    clock.insert(1, 50); // w0
    clock.insert(2, 300); // w1
    clock.insert(3, 20_000); // w2
    clock.insert(4, 1_100_000); // w3

    // Advance to collect all
    let result = clock.advance(1_100_000);

    assert_eq!(result.len(), 4, "Should collect from all wheels");
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
    assert!(result.contains(&4));
  }

  #[test]
  fn test_clock_insert_boundary_values() {
    let mut clock: Clock<i32> = logical_clock();

    // Test boundary values between wheels
    clock.insert(1, 255); // Last slot of w0
    clock.insert(2, 256); // First slot of w1
    clock.insert(3, 16_383); // Last slot of w1 range
    clock.insert(4, 16_384); // First slot of w2

    let result = clock.advance(16384);

    assert_eq!(result.len(), 4, "Should handle boundary values correctly");
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
    assert!(result.contains(&4));
  }

  #[test]
  fn test_clock_insert_and_advance_partial() {
    let mut clock: Clock<i32> = logical_clock();

    clock.insert(1, 10);
    clock.insert(2, 20);
    clock.insert(3, 30);

    // Advance only partway
    let result1 = clock.advance(15);
    assert_eq!(result1.len(), 1, "Should get first item");
    assert!(result1.contains(&1));

    // Advance to get second item
    let result2 = clock.advance(5);
    assert_eq!(result2.len(), 1, "Should get second item");
    assert!(result2.contains(&2));

    // Advance to get third item
    let result3 = clock.advance(10);
    assert_eq!(result3.len(), 1, "Should get third item");
    assert!(result3.contains(&3));
  }

  #[test]
  fn test_clock_insert_zero_duration() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert with duration 0 (should go to current slot in w0)
    clock.insert(42, 0);

    // Should not be returned immediately (needs at least one tick)
    let result = clock.advance(0);
    assert_eq!(result.len(), 0, "Duration 0 should not fire immediately");
  }

  #[test]
  fn test_clock_insert_respects_wheel_hierarchy() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert a timer at exactly W0_SIZE (256) - should go to w1
    let w0_size_duration = W0_SIZE;
    clock.insert(100, w0_size_duration);

    // Insert a timer at W0_SIZE * W1_SIZE - should go to w2
    let w1_size_duration = W0_SIZE * W1_SIZE;
    clock.insert(200, w1_size_duration);

    // Advance to collect both
    let result = clock.advance(w1_size_duration);

    assert!(result.contains(&100), "Should collect w1 item");
    assert!(result.contains(&200), "Should collect w2 item");
  }

  // Tests for cascading/demotion
  #[test]
  fn test_clock_cascading_from_w1_to_w0() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert timer with duration 300 (256 + 44)
    // Should go to w1, then cascade to w0 after 256 ticks
    clock.insert(42, 300);

    // Advance 256 units - should NOT fire yet (item should cascade to w0)
    let result1 = clock.advance(256);
    assert_eq!(
      result1.len(),
      0,
      "Timer should not fire at 256, needs cascading to w0"
    );

    // Advance 44 more units - NOW it should fire
    let result2 = clock.advance(44);
    assert_eq!(result2.len(), 1, "Timer should fire after total 300 units");
    assert_eq!(result2[0], 42);
  }

  #[test]
  fn test_clock_cascading_precise_timing() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert multiple timers with durations > W0_SIZE
    clock.insert(1, 300); // 256 + 44
    clock.insert(2, 500); // 256 + 244
    clock.insert(3, 1_000); // 3*256 + 232

    // Advance exactly 300 - should get first timer only
    let result1 = clock.advance(300);
    assert_eq!(result1.len(), 1, "Should get timer at 300");
    assert!(result1.contains(&1));

    // Advance to 500 total (200 more) - should get second timer
    let result2 = clock.advance(200);
    assert_eq!(result2.len(), 1, "Should get timer at 500");
    assert!(result2.contains(&2));

    // Advance to 1000 total (500 more) - should get third timer
    let result3 = clock.advance(500);
    assert_eq!(result3.len(), 1, "Should get timer at 1000");
    assert!(result3.contains(&3));
  }

  #[test]
  fn test_clock_cascading_from_w2() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert timer in w2 range: 20000 = (1 << 14) + some remainder
    // Should cascade through w1 to w0
    clock.insert(99, 20_000);

    // Should not fire before 20000 units
    let result1 = clock.advance(19_999);
    assert_eq!(result1.len(), 0, "Should not fire before 20000");

    // Should fire at exactly 20000
    let result2 = clock.advance(1);
    assert_eq!(result2.len(), 1, "Should fire at 20000");
    assert_eq!(result2[0], 99);
  }

  #[test]
  fn test_clock_no_premature_firing_from_higher_wheels() {
    let mut clock: Clock<i32> = logical_clock();

    // Insert timer that will be in w1
    clock.insert(100, 260);

    // Advance just past w0 wrap (256) but not enough for timer (260)
    let result1 = clock.advance(256);
    assert_eq!(result1.len(), 0, "Timer at 260 should not fire at 256");

    // Advance 4 more to reach 260
    let result2 = clock.advance(4);
    assert_eq!(result2.len(), 1, "Timer should fire at 260");
    assert_eq!(result2[0], 100);
  }
}

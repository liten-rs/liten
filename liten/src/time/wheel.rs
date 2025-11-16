pub struct Wheel<const LEN: usize, I> {
  slots: [Vec<I>; LEN], // Fixed-size array of Vec<Timer>
  current_slot: usize,
}

pub struct TimerTickResult<T> {
  pub slots: Vec<T>,
  pub resetted_counter: usize,
}

impl<const L: usize, I> Wheel<L, I> {
  pub fn peak_nearest(&self) -> Option<usize> {
    let current_slot = self.current_slot;
    // TODO: Could be improved
    for index in current_slot + 1..L {
      let slot = self.slots.get(index).unwrap();
      if !slot.is_empty() {
        return Some(index - current_slot);
      }
    }
    None
  }

  pub fn new_with_position(position: usize) -> Self {
    assert!(
      position <= L,
      "Tried to create wheel with position greater than wheel size"
    );

    let slots = std::array::from_fn::<Vec<I>, L, _>(|_| Vec::<I>::new());
    Wheel { slots, current_slot: position }
  }

  pub fn insert(&mut self, tick_forward: usize, value: I) {
    assert!(L >= tick_forward);
    let idx = (self.current_slot + tick_forward) % L;

    self.slots[idx].push(value);
  }

  /// Advances the wheel and returns and pops the slots
  pub fn advance(&mut self, ticks: usize) -> TimerTickResult<I> {
    let mut slots = Vec::new();

    let how_many_carry_over = (self.current_slot + ticks) / L;
    let ticks_ = if how_many_carry_over > 0 { L } else { ticks };

    for time in 1..=ticks_ {
      let new_current_slot = (self.current_slot + time) % L;

      let thing = std::mem::take(self.slots.get_mut(new_current_slot).unwrap());

      slots.extend(thing);
    }

    self.current_slot = (self.current_slot + ticks) % L;

    TimerTickResult { slots, resetted_counter: how_many_carry_over }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_wheel_with_position_zero() {
    let wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    assert_eq!(wheel.peak_nearest(), None);
  }

  #[test]
  fn test_new_wheel_with_position() {
    let wheel: Wheel<10, i32> = Wheel::new_with_position(5);
    assert_eq!(wheel.peak_nearest(), None);
  }

  #[test]
  #[should_panic(
    expected = "Tried to create wheel with position greater than wheel size"
  )]
  fn test_new_wheel_with_invalid_position() {
    let _wheel: Wheel<10, i32> = Wheel::new_with_position(11);
  }

  #[test]
  fn test_carryover() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 50);

    let nearest = wheel.peak_nearest();
    assert_eq!(nearest, Some(1));

    wheel.advance(99);
  }

  #[test]
  fn test_insert_single_item() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 42);

    let nearest = wheel.peak_nearest();
    assert_eq!(nearest, Some(1));
  }

  #[test]
  fn test_insert_multiple_items_same_slot() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(3, 1);
    wheel.insert(3, 2);
    wheel.insert(3, 3);

    let nearest = wheel.peak_nearest();
    assert_eq!(nearest, Some(3));
  }

  #[test]
  fn test_insert_multiple_items_different_slots() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(5, 1);
    wheel.insert(3, 2);
    wheel.insert(7, 3);

    let nearest = wheel.peak_nearest();
    assert_eq!(nearest, Some(3));
  }

  #[test]
  #[should_panic]
  fn test_insert_beyond_wheel_size() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(11, 42);
  }

  #[test]
  fn test_advance_one_tick_no_items() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    let result = wheel.advance(1);

    assert_eq!(result.slots.len(), 0);
    assert_eq!(result.resetted_counter, 0);
  }

  #[test]
  fn test_advance_one_tick_with_item() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 42);

    let result = wheel.advance(1);

    assert_eq!(result.slots.len(), 1);
    assert_eq!(result.slots[0], 42);
    assert_eq!(result.resetted_counter, 0);
  }

  #[test]
  fn test_advance_multiple_ticks() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 10);
    wheel.insert(2, 20);
    wheel.insert(3, 30);

    let result = wheel.advance(3);

    assert_eq!(result.slots.len(), 3);
    assert!(result.slots.contains(&10));
    assert!(result.slots.contains(&20));
    assert!(result.slots.contains(&30));
    assert_eq!(result.resetted_counter, 0);
  }

  #[test]
  fn test_advance_past_items() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(2, 42);

    // Advance past the item
    let result = wheel.advance(5);

    assert_eq!(result.slots.len(), 1);
    assert_eq!(result.slots[0], 42);
    assert_eq!(result.resetted_counter, 0);
  }

  #[test]
  fn test_wheel_wrap_around() {
    let mut wheel: Wheel<5, i32> = Wheel::new_with_position(0);

    // Advance enough to wrap around once
    let result = wheel.advance(5);

    assert_eq!(result.resetted_counter, 1);
    assert_eq!(result.slots.len(), 0);
  }

  #[test]
  fn test_wheel_multiple_wrap_arounds() {
    let mut wheel: Wheel<5, i32> = Wheel::new_with_position(0);

    // Advance enough to wrap around twice
    let result = wheel.advance(10);

    assert_eq!(result.resetted_counter, 2);
    assert_eq!(result.slots.len(), 0);
  }

  #[test]
  fn test_wheel_wrap_around_with_items() {
    let mut wheel: Wheel<5, i32> = Wheel::new_with_position(4);
    wheel.insert(2, 100);

    // This should wrap around: position 4 + 2 ticks = slot 1 (wrapping at 5)
    let result = wheel.advance(2);

    assert_eq!(result.resetted_counter, 1);
    assert_eq!(result.slots.len(), 1);
    assert_eq!(result.slots[0], 100);
  }

  #[test]
  fn test_peak_nearest_empty_wheel() {
    let wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    assert_eq!(wheel.peak_nearest(), None);
  }

  #[test]
  fn test_peak_nearest_with_items() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(2);
    wheel.insert(1, 1);
    wheel.insert(5, 2);
    wheel.insert(3, 3);

    // Should return the nearest slot relative to current_slot (2)
    assert_eq!(wheel.peak_nearest(), Some(1));
  }

  #[test]
  fn test_peak_nearest_after_advance() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(2, 1);
    wheel.insert(5, 2);

    assert_eq!(wheel.peak_nearest(), Some(2));

    // Advance past first item
    wheel.advance(3);

    // Now nearest should be 2 slots away (slot 5 - current slot 3)
    assert_eq!(wheel.peak_nearest(), Some(2));
  }

  #[test]
  fn test_peak_nearest_no_items_ahead() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(5);
    wheel.insert(2, 1);

    // Item is at slot 7, current is 5, so distance is 2
    assert_eq!(wheel.peak_nearest(), Some(2));

    // Advance past the item
    wheel.advance(3);

    // Now at slot 8, no items ahead (only checks current_slot+1 to L)
    assert_eq!(wheel.peak_nearest(), None);
  }

  #[test]
  fn test_insert_at_zero_ticks() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(3);
    wheel.insert(0, 42);

    // Item inserted at current slot (3 + 0 = 3)
    // peak_nearest checks from current_slot + 1, so won't see it
    assert_eq!(wheel.peak_nearest(), None);
  }

  #[test]
  fn test_advance_returns_items_in_order_of_advancement() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 100);
    wheel.insert(2, 200);
    wheel.insert(1, 101); // Same slot as first item

    let result = wheel.advance(2);

    // Should get both items from slot 1, then item from slot 2
    assert_eq!(result.slots.len(), 3);
    // First two should be from slot 1
    assert!(result.slots[0] == 100 || result.slots[0] == 101);
    assert!(result.slots[1] == 100 || result.slots[1] == 101);
    assert_eq!(result.slots[2], 200);
  }

  #[test]
  fn test_wheel_with_string_items() {
    let mut wheel: Wheel<5, String> = Wheel::new_with_position(0);
    wheel.insert(1, "first".to_string());
    wheel.insert(2, "second".to_string());

    let result = wheel.advance(2);

    assert_eq!(result.slots.len(), 2);
    assert_eq!(result.slots[0], "first");
    assert_eq!(result.slots[1], "second");
  }

  #[test]
  fn test_wheel_position_boundary() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(9);
    wheel.insert(1, 42);

    // Should wrap: position 9 + 1 = slot 10 % 10 = slot 0
    let result = wheel.advance(1);

    assert_eq!(result.resetted_counter, 1);
    assert_eq!(result.slots.len(), 1);
    assert_eq!(result.slots[0], 42);
  }

  #[test]
  fn test_consecutive_advances() {
    let mut wheel: Wheel<10, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 1);
    wheel.insert(3, 3);
    wheel.insert(5, 5);

    let result1 = wheel.advance(2);
    assert_eq!(result1.slots, vec![1]);

    let result2 = wheel.advance(2);
    assert_eq!(result2.slots, vec![3]);

    let result3 = wheel.advance(2);
    assert_eq!(result3.slots, vec![5]);
  }

  #[test]
  fn test_large_wheel() {
    let mut wheel: Wheel<1000, i32> = Wheel::new_with_position(0);
    wheel.insert(500, 42);
    wheel.insert(999, 99);

    assert_eq!(wheel.peak_nearest(), Some(500));

    let result = wheel.advance(500);
    assert_eq!(result.slots, vec![42]);

    assert_eq!(wheel.peak_nearest(), Some(499));
  }

  #[test]
  fn test_timer_tick_result_structure() {
    let mut wheel: Wheel<5, i32> = Wheel::new_with_position(0);
    wheel.insert(1, 42);

    let TimerTickResult { slots, resetted_counter } = wheel.advance(1);

    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0], 42);
    assert_eq!(resetted_counter, 0);
  }

  #[test]
  fn test_insert_wrapping_behavior() {
    let mut wheel: Wheel<5, i32> = Wheel::new_with_position(3);
    wheel.insert(3, 42);

    // position 3 + 3 = 6, 6 % 5 = slot 1
    // Need to advance 3 ticks to reach it
    let result = wheel.advance(3);

    assert_eq!(result.slots.len(), 1);
    assert_eq!(result.slots[0], 42);
  }

  #[test]
  fn test_multiple_items_across_wrap() {
    let mut wheel: Wheel<5, i32> = Wheel::new_with_position(3);
    wheel.insert(1, 10); // slot 4
    wheel.insert(2, 20); // slot 0 (wraps)
    wheel.insert(3, 30); // slot 1 (wraps)

    let result = wheel.advance(5);

    assert_eq!(result.slots.len(), 3);
    assert_eq!(result.resetted_counter, 1);
    // Verify all items were returned
    assert!(result.slots.contains(&10));
    assert!(result.slots.contains(&20));
    assert!(result.slots.contains(&30));
  }
}

use std::fmt;

pub struct Wheel<const T: usize, I> {
  slots: [Vec<I>; T], // Fixed-size array of Vec<Timer>
  current_slot: usize,
}

impl<const T: usize, I> fmt::Debug for Wheel<T, I> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Wheel")
      .field("slot_sizes", &"[Cell<Vec<I>>; T]")
      .field("current_slot", &self.current_slot)
      .finish()
  }
}

pub struct TimerTickResult<T> {
  pub slots: Vec<T>,
  pub resetted_counter: usize,
}

impl<const T: usize, I> Wheel<T, I> {
  pub fn peak_nearest(&self) -> Option<usize> {
    let current_slot = self.current_slot;
    // TODO: Could be improved
    for index in current_slot + 1..T {
      let slot = self.slots.get(index).unwrap();
      if !slot.is_empty() {
        return Some(index - current_slot);
      }
    }
    None
  }
  /// Overflow will be wrapped
  pub fn new_with_position(position: usize) -> Self {
    let slots = std::array::from_fn::<Vec<I>, T, _>(|_| Vec::<I>::new());
    Wheel { slots, current_slot: position % T }
  }

  pub fn insert(&mut self, tick_forward: usize, value: I) {
    let idx = (self.current_slot + tick_forward) % T;

    self.slots[idx].push(value);
  }

  /// Advances the wheel and returns and pops the slots
  pub fn advance(&mut self, ticks: usize) -> TimerTickResult<I> {
    let mut slots = Vec::new();
    let mut how_many_carry_over = 0;
    for _ in 0..ticks {
      let new_current_slot = (self.current_slot + 1) % T;
      if new_current_slot == 0 {
        how_many_carry_over += 1;
      }
      self.current_slot = new_current_slot;

      let thing = std::mem::replace(
        self.slots.get_mut(new_current_slot).unwrap(),
        Vec::new(),
      );

      slots.extend(thing);
    }

    TimerTickResult { slots, resetted_counter: how_many_carry_over }
  }
}

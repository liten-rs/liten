use std::{cell::Cell, fmt};

use super::clock::Timer;

pub struct Wheel<const T: usize, I> {
  slots: [Cell<Vec<I>>; T], // Fixed-size array of Vec<Timer>
  current_slot: Cell<usize>,
}

impl<const T: usize, I> fmt::Debug for Wheel<T, I>
where
  I: Clone, // Needed because we have to clone Vec<I> to put it back
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // let sizes: Vec<usize> = self
    //   .slots
    //   .iter()
    //   .map(|cell| {
    //     let vec = cell.take();
    //     let len = vec.len();
    //     cell.set(vec); // Put it back
    //     len
    //   })
    //   .collect();
    f.debug_struct("Wheel")
      .field("slot_sizes", &"[Cell<Vec<I>>; T]")
      .field("current_slot", &self.current_slot.get())
      .finish()
  }
}

pub struct TimerTickResult<T> {
  pub slots: Vec<T>,
  pub resetted_counter: usize,
}

impl<const T: usize> Wheel<T, Timer> {
  pub fn peak_nearest(&mut self) -> Option<usize> {
    for index in (self.current_slot.get() + 1..T) {
      if !self.slots.get_mut(index).unwrap().get_mut().is_empty() {
        return Some(index);
      }
    }
    None
  }
}

impl<const T: usize, I> Wheel<T, I> {
  // pub fn new() -> Self {
  //   Self::new_with_position(0)
  // }

  /// Overflow will be wrapped
  pub fn new_with_position(position: usize) -> Self {
    let slots =
      std::array::from_fn::<Cell<Vec<I>>, T, _>(|_| Cell::new(Vec::<I>::new()));
    Wheel { slots, current_slot: Cell::new(position % T) }
  }

  pub fn insert(&self, tick_forward: usize, value: I) {
    let idx = self.current_slot.get() + tick_forward;

    let mut vec = self.slots[idx].take();
    vec.push(value);

    self.slots[idx].set(vec);
  }

  /// Advances the wheel and returns and pops the slots
  pub fn advance(&self, ticks: usize) -> TimerTickResult<I> {
    let mut slots = Vec::new();
    let mut how_many_carry_over = 0;
    for _ in 0..ticks {
      let new_current_slot = (self.current_slot.get() + 1) % T;
      if new_current_slot == 0 {
        how_many_carry_over += 1;
      }
      self.current_slot.set(new_current_slot);

      slots.extend(self.slots[new_current_slot].take());
    }

    TimerTickResult { slots, resetted_counter: how_many_carry_over }
  }
}

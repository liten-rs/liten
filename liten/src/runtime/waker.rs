use std::mem::ManuallyDrop;
use std::task::{RawWaker, RawWakerVTable, Waker};

use std::{sync::Arc, thread};

static RUNTIME_WAKER_VTABLE: RawWakerVTable =
  RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

pub fn park_waker(unparker: thread::Thread) -> Waker {
  let state = Arc::into_raw(Arc::new(RuntimeWakerData(unparker)));
  unsafe { Waker::new(state as *const (), &RUNTIME_WAKER_VTABLE) }
}

unsafe fn waker_clone(data: *const ()) -> RawWaker {
  unsafe {
    Arc::increment_strong_count(data as *const RuntimeWakerData);
  };

  RawWaker::new(data, &RUNTIME_WAKER_VTABLE)
}
unsafe fn waker_wake(data: *const ()) {
  let data = unsafe { Arc::from_raw(data as *const RuntimeWakerData) };

  data.0.unpark();
}
unsafe fn waker_wake_by_ref(data: *const ()) {
  let data = unsafe {
    ManuallyDrop::new(Arc::from_raw(data as *const RuntimeWakerData))
  };

  data.0.unpark();
}
unsafe fn waker_drop(data: *const ()) {
  unsafe { Arc::decrement_strong_count(data as *const RuntimeWakerData) };
}

// Waker implementation to notify the runtime
struct RuntimeWakerData(thread::Thread);

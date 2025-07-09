use std::mem::ManuallyDrop;
use std::task::{RawWaker, RawWakerVTable, Waker};

use parking::Unparker;

use crate::loom::sync::Arc;
use crate::task::{TaskId, TaskStore};

pub struct TaskWakerData {
  unparker: Unparker,
  task_id: TaskId,
}

static TASK_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
  task_waker_clone,
  task_waker_wake,
  task_waker_wake_by_ref,
  task_waker_drop,
);

pub fn create_task_waker(unparker: Unparker, task_id: TaskId) -> Waker {
  let state = Arc::into_raw(Arc::new(TaskWakerData { unparker, task_id }));
  unsafe { Waker::new(state as *const (), &TASK_WAKER_VTABLE) }
}

unsafe fn task_waker_clone(data: *const ()) -> RawWaker {
  unsafe {
    Arc::increment_strong_count(data as *const TaskWakerData);
  };

  RawWaker::new(data, &TASK_WAKER_VTABLE)
}
unsafe fn task_waker_wake(data: *const ()) {
  let data = unsafe { Arc::from_raw(data as *const TaskWakerData) };

  TaskStore::get().wake_task(data.task_id);
  data.unparker.unpark();
}
unsafe fn task_waker_wake_by_ref(data: *const ()) {
  let data =
    unsafe { ManuallyDrop::new(Arc::from_raw(data as *const TaskWakerData)) };

  TaskStore::get().wake_task(data.task_id);
  data.unparker.unpark();
}
unsafe fn task_waker_drop(data: *const ()) {
  unsafe { Arc::decrement_strong_count(data as *const TaskWakerData) };
}

use std::mem::ManuallyDrop;
use std::task::{RawWaker, RawWakerVTable, Waker};

use parking::Unparker;

use crate::loom::thread::Thread;

use crate::loom::sync::Arc;
use crate::task::TaskId;
use std::sync::mpsc;

pub struct TaskWakerData {
  task_id: TaskId,
  sender: mpsc::Sender<TaskId>,
  unparker: Unparker,
}

static TASK_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
  task_waker_clone,
  task_waker_wake,
  task_waker_wake_by_ref,
  task_waker_drop,
);

pub fn create_task_waker(
  task_id: TaskId,
  sender: mpsc::Sender<TaskId>,
  unparker: Unparker,
) -> Waker {
  let state =
    Arc::into_raw(Arc::new(TaskWakerData { task_id, sender, unparker }));
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

  let _ = data.sender.send(data.task_id);
  data.unparker.unpark();
}
unsafe fn task_waker_wake_by_ref(data: *const ()) {
  let data =
    unsafe { ManuallyDrop::new(Arc::from_raw(data as *const TaskWakerData)) };

  let _ = data.sender.send(data.task_id);
  data.unparker.unpark();
}
unsafe fn task_waker_drop(data: *const ()) {
  unsafe { Arc::decrement_strong_count(data as *const TaskWakerData) };
}

static RUNTIME_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
  runtime_waker_clone,
  runtime_waker_wake,
  runtime_waker_wake_by_ref,
  runtime_waker_drop,
);

pub fn create_runtime_waker(thread: Thread) -> Waker {
  let state = Arc::into_raw(Arc::new(RuntimeWakerData(thread)));
  unsafe { Waker::new(state as *const (), &RUNTIME_WAKER_VTABLE) }
}

unsafe fn runtime_waker_clone(data: *const ()) -> RawWaker {
  unsafe {
    Arc::increment_strong_count(data as *const RuntimeWakerData);
  };

  RawWaker::new(data, &RUNTIME_WAKER_VTABLE)
}
unsafe fn runtime_waker_wake(data: *const ()) {
  let data = unsafe { Arc::from_raw(data as *const RuntimeWakerData) };

  data.0.unpark();
}
unsafe fn runtime_waker_wake_by_ref(data: *const ()) {
  let data = unsafe {
    ManuallyDrop::new(Arc::from_raw(data as *const RuntimeWakerData))
  };

  data.0.unpark();
}
unsafe fn runtime_waker_drop(data: *const ()) {
  unsafe { Arc::decrement_strong_count(data as *const RuntimeWakerData) };
}

// Waker implementation to notify the runtime
pub struct RuntimeWakerData(Thread);

use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::loom::sync::Arc;
use crate::task::task::raw::RawTask;
use crate::task::Task;

pub struct TaskWakerData<Fun> {
  // unparker: Unparker,
  // schedule: fn(Task),
  // task: *const RawTask<Fun>,
}

static TASK_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
  task_waker_clone,
  task_waker_wake,
  task_waker_wake_by_ref,
  task_waker_drop,
);

pub fn create_task_waker<Fun>(task: *const RawTask<Fun>) -> Waker {
  let state = Arc::into_raw(Arc::new(TaskWakerData { task }));
  unsafe { Waker::new(state as *const (), &TASK_WAKER_VTABLE) }
}

unsafe fn task_waker_clone(data: *const ()) -> RawWaker {
  unsafe {
    Arc::increment_strong_count(data as *const TaskWakerData);
  };

  RawWaker::new(data, &TASK_WAKER_VTABLE)
}
unsafe fn task_waker_wake<Fun>(data: *const ()) {
  let data = unsafe { Arc::from_raw(data as *const TaskWakerData<Fun>) };

  let task = unsafe { (data.task as *const RawTask<Fun>).as_ref() }.unwrap();

  // task.schedule();

  // unsafe { task.as_ref().unwrap() }.schedule();

  // unsafe { task.as_ref().unwrap() }.schedule();

  // (data.schedule)();

  // tracing::trace!(task_id = ?&data.task_id, "waker activated");

  // let _ = TaskStore::get().insert(&data.task_id);
  // data.unparker.unpark();
}
unsafe fn task_waker_wake_by_ref(data: *const ()) {
  task_waker_wake(data);
  // let data =
  //   unsafe { ManuallyDrop::new(Arc::from_raw(data as *const TaskWakerData)) };

  // tracing::trace!(task_id = ?&data.task_id, "waker activated");

  // let _ = TaskStore::get().move_to_hot(&data.task_id);
  // data.unparker.unpark();
}
unsafe fn task_waker_drop(data: *const ()) {
  unsafe { Arc::decrement_strong_count(data as *const TaskWakerData) };
}

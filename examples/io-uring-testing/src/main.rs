#![allow(dead_code)]
use io_uring::{IoUring, opcode, types};
use std::collections::HashMap;
use std::fs::File;
use std::mem::forget;
use std::os::fd::{AsFd, BorrowedFd, RawFd};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::{fs, io, thread};

static NEWEST_INDEX: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct IoUringHandle {
  job_sender: Sender<IoUringPayload>,
}

pub struct IoUringPayload {
  operation: IoUringOperation,
  reply: Sender<Vec<u8>>,
}

pub struct IoUringState {
  job_receiver: Receiver<IoUringPayload>,
}

#[derive(Debug)]
pub struct WorkerOperation {
  mem: Vec<u8>,
  len: usize,
}

fn io_uring_background_thread(state: IoUringState) {
  let mut store = HashMap::new();
  let mut io_uring = IoUring::new(256).unwrap();
  let (submitter, mut submit_queue, mut completion) = io_uring.split();

  for item in state.job_receiver.into_iter() {
    // let addr: *mut u8;
    let (entry, mem) = match item.operation {
      IoUringOperation::Read(read) => {
        println!("Creating read operation for fd: {}, len: {}", read.fd, read.len);
        
        let mut mem = Vec::with_capacity(read.len as usize);

        for _ in 0..read.len as usize {
          mem.push(0);
        }

        println!("Buffer created, ptr: {:?}, len: {}", mem.as_mut_ptr(), mem.len());
        
        // Check if the file descriptor is valid
        println!("File descriptor: {}, is valid: {}", read.fd, read.fd >= 0);
        
        // Check memory alignment
        let ptr = mem.as_mut_ptr() as usize;
        println!("Buffer alignment: ptr={}, aligned_4k={}", ptr, ptr % 4096 == 0);

        let entry = opcode::Read::new(
          types::Fd(read.fd),
          mem.as_mut_ptr(),
          mem.len() as _,
        )
        .build();

        (entry, mem)
      }
    };
    println!("nice");

    let idx = NEWEST_INDEX.fetch_add(1, Ordering::AcqRel);
    println!("nice");
    let entry = entry.user_data(idx);
    println!("nice");

    // let addr = addr as *mut Vec<u8>;

    // let l = unsafe { &*(addr) };

    // println!("testinb {:#?}");

    store.insert(idx, WorkerOperation { len: mem.len(), mem: mem });
    unsafe {
      submit_queue.push(&entry).expect("submission queue is full");
    }
    println!("nice");

    // Submit the request and wait for completion
    println!("Submitting request and waiting for completion...");
    
    // Try with a timeout first to see if it's just slow
    let submit_result = match submitter.submit_and_wait_timeout(1, std::time::Duration::from_secs(5)) {
        Ok(result) => {
            println!("Submit and wait with timeout succeeded: {:?}", result);
            Ok(result)
        }
        Err(e) => {
            println!("Submit and wait with timeout failed: {:?}", e);
            // Fall back to regular submit_and_wait
            let result = submitter.submit_and_wait(1);
            println!("Submit and wait result: {:?}", result);
            result
        }
    };
    submit_result.unwrap();

    println!("Getting completion...");
    // Get the completion that was just submitted
    let value = match completion.next() {
        Some(v) => {
            println!("Got completion: {:?}", v);
            v
        }
        None => {
            println!("No completion available!");
            panic!("No completion available");
        }
    };

    println!("nice");
    let id = value.user_data();
    println!("nice");
    let op = store.remove(&id).unwrap();
    // println!("{} {:?}", id, &op);

    item.reply.send(op.mem).unwrap();
  }
}
fn is_aligned_for_type<T>(ptr: *const u8) -> bool {
  let align = std::mem::align_of::<T>();
  let addr = ptr as usize;
  addr % align == 0
}

pub enum IoUringOperation {
  Read(ReadOperation),
}
pub struct ReadOperation {
  len: u64,
  fd: RawFd,

  // So the file descriptor doesn't get removed
  _f: File,
}

impl ReadOperation {
  pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
    let path = path.as_ref();
    let file = fs::File::open(&path)?;
    let meta = fs::metadata(&path).unwrap();

    Ok(Self { fd: file.as_raw_fd(), len: meta.len(), _f: file })
  }

  pub fn read_file(self, handle: IoUringHandle) -> io::Result<Vec<u8>> {
    let io_uring_op = IoUringOperation::Read(self);

    let (sender, receiver) = mpsc::channel();

    handle
      .job_sender
      .send(IoUringPayload { operation: io_uring_op, reply: sender })
      .unwrap();
    println!("sent");

    let result = receiver.recv().unwrap();
    println!("received");

    Ok(result)
  }
}

fn create_io_uring() -> (IoUringState, IoUringHandle) {
  let (job_sender, job_receiver) = mpsc::channel::<IoUringPayload>();
  (IoUringState { job_receiver }, IoUringHandle { job_sender })
}

fn main() -> io::Result<()> {
  let (state, handle) = create_io_uring();
  thread::spawn(move || io_uring_background_thread(state));

  let read_operation = ReadOperation::new("./README.md")?;

  let result = read_operation.read_file(handle.clone());

  println!("{:?}", String::from_utf8(result.unwrap()).unwrap());

  Ok(())
}

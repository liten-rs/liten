use criterion::{criterion_group, criterion_main, Criterion};
use liten::sync::mpsc;

fn test_channel(sender: mpsc::Sender<u8>, receiver: mpsc::Receiver<u8>) {
  let times = 2000;

  for _ in 0..times {
    sender.send(0u8).unwrap();
  }

  for _ in 0..times {
    receiver.try_recv().unwrap();
  }
}

fn test_std(
  sender: std::sync::mpsc::Sender<u8>,
  receiver: std::sync::mpsc::Receiver<u8>,
) {
  let times = 2000;

  for _ in 0..times {
    sender.send(0u8).unwrap();
  }

  for _ in 0..times {
    receiver.try_recv().unwrap();
  }
}

fn test_std2(
  sender: std::sync::mpsc::SyncSender<u8>,
  receiver: std::sync::mpsc::Receiver<u8>,
) {
  let times = 2000;

  for _ in 0..times {
    sender.send(0u8).unwrap();
  }

  for _ in 0..times {
    receiver.try_recv().unwrap();
  }
}

fn criterion_benchmark(c: &mut Criterion) {
  let mut liten_channel = c.benchmark_group("liten::sync::channel");
  liten_channel.bench_function("starter-capacity", |b| {
    b.iter(|| {
      let (sender, receiver) = mpsc::unbounded();
      test_channel(sender, receiver)
    })
  });
  liten_channel.bench_function("capacity-2048", |b| {
    b.iter(|| {
      let (sender, receiver) = mpsc::unbounded_with_capacity(2048);
      test_channel(sender, receiver)
    })
  });

  drop(liten_channel);

  let mut std_channel = c.benchmark_group("std::mpsc::channel");

  std_channel.bench_function("starter-capacity", |b| {
    b.iter(|| {
      let (sender, receiver) = std::sync::mpsc::channel();
      test_std(sender, receiver);
    })
  });

  std_channel.bench_function("capacity-2048", |b| {
    b.iter(|| {
      let (sender, receiver) = std::sync::mpsc::sync_channel(2048);
      test_std2(sender, receiver);
    })
  });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

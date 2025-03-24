use liten::sync::mpsc;

#[cfg(not(loom))]
#[test]
fn iter() {
  let (sender, receiver) = mpsc::unbounded::<i32>();

  sender.send(1).unwrap();
  sender.send(1).unwrap();
  sender.send(1).unwrap();
  sender.send(1).unwrap();
  sender.send(1).unwrap();
  sender.send(1).unwrap();
  sender.send(1).unwrap();

  let vec: Vec<i32> = receiver.try_iter().collect();

  assert_eq!(vec.len(), 7);
}

#[cfg(not(loom))]
#[test]
fn mpsc() {
  liten::runtime::Runtime::new().block_on(async {
    let (sender, receiver) = mpsc::unbounded::<i32>();

    sender.send(1).unwrap();
    sender.send(1).unwrap();
    sender.send(1).unwrap();
    sender.send(1).unwrap();
    sender.send(1).unwrap();
    sender.send(1).unwrap();
    sender.send(1).unwrap();

    let vec: i32 = receiver.recv().await.unwrap();

    assert_eq!(vec, 1);

    let vec: Vec<i32> = receiver.try_iter().collect();

    assert_eq!(vec.len(), 6);
  })
}

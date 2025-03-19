use liten::sync::mpsc;

#[liten::test]
async fn iter() {
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

#[liten::test]
async fn mpsc() {
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
}

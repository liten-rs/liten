// use liten::sync::mpsc;
//
// #[liten::internal_test]
// fn iter() {
//   let (sender, receiver) = mpsc::unbounded::<i32>();
//
//   sender.send(1).unwrap();
//   sender.send(1).unwrap();
//   sender.send(1).unwrap();
//   sender.send(1).unwrap();
//   sender.send(1).unwrap();
//   sender.send(1).unwrap();
//   sender.send(1).unwrap();
//
//   let vec: Vec<i32> = receiver.try_iter().collect();
//
//   assert_eq!(vec.len(), 7);
// }
//
// #[liten::internal_test]
// fn sender_testing() {
//   let (sender, receiver) = liten::sync::mpsc::unbounded::<i32>();
//
//   let sender_1 = sender.clone();
//   let sender_2 = sender.clone();
//
//   sender_1.send(1).unwrap();
//   sender_1.send(2).unwrap();
//   sender_1.send(3).unwrap();
//   assert_eq!(receiver.try_recv().unwrap(), 1);
//
//   sender_2.send(4).unwrap();
//   sender_2.send(5).unwrap();
//   sender_2.send(6).unwrap();
//
//   assert!(receiver.try_recv().unwrap() == 2);
//   assert!(receiver.try_recv().unwrap() == 3);
//   assert_eq!(receiver.try_recv().unwrap(), 4);
//   assert!(receiver.try_recv().unwrap() == 5);
//   assert!(receiver.try_recv().unwrap() == 6);
//   assert!(receiver.try_recv() == Err(liten::sync::mpsc::RecvError::Empty));
// }
//
// #[liten::test]
// async fn async_testing() {
//   let (sender, receiver) = liten::sync::mpsc::unbounded::<i32>();
//
//   let sender_1 = sender.clone();
//   let sender_2 = sender.clone();
//
//   sender_1.send(1).unwrap();
//   sender_1.send(2).unwrap();
//   sender_1.send(3).unwrap();
//   assert_eq!(receiver.try_recv().unwrap(), 1);
//
//   sender_2.send(4).unwrap();
//   sender_2.send(5).unwrap();
//   sender_2.send(6).unwrap();
//
//   assert!(receiver.recv().await.unwrap() == 2);
//   assert!(receiver.recv().await.unwrap() == 3);
//   assert_eq!(receiver.recv().await.unwrap(), 4);
//   assert!(receiver.recv().await.unwrap() == 5);
//   assert!(receiver.recv().await.unwrap() == 6);
//   assert!(receiver.recv().await == Err(liten::sync::mpsc::RecvError::Empty));
// }

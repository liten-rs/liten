#![cfg(loom)]

use liten::sync::mpsc;

#[test]
fn iter() {
  loom::model(|| {
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
  })
}

//#[test]
//fn mpsc() {
//  loom::model(|| {
//    liten::runtime::Runtime::builder().num_workers(1).block_on(async {
//      let (sender, receiver) = mpsc::unbounded::<i32>();
//
//      sender.send(1).unwrap();
//      sender.send(1).unwrap();
//      sender.send(1).unwrap();
//      sender.send(1).unwrap();
//      sender.send(1).unwrap();
//      sender.send(1).unwrap();
//      sender.send(1).unwrap();
//
//      let vec: i32 = receiver.recv().await.unwrap();
//
//      assert_eq!(vec, 1);
//
//      let vec: Vec<i32> = receiver.try_iter().collect();
//
//      assert_eq!(vec.len(), 6);
//    })
//  })
//}

#[test]
fn sender_testing() {
  loom::model(|| {
    let (sender, receiver) = liten::sync::mpsc::unbounded::<i32>();

    let sender_1 = sender.clone();
    let sender_2 = sender.clone();

    sender_1.send(1).unwrap();
    sender_1.send(2).unwrap();
    sender_1.send(3).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), 1);

    sender_2.send(4).unwrap();
    sender_2.send(5).unwrap();
    sender_2.send(6).unwrap();

    assert!(receiver.try_recv().unwrap() == 2);
    assert!(receiver.try_recv().unwrap() == 3);
    assert_eq!(receiver.try_recv().unwrap(), 4);
    assert!(receiver.try_recv().unwrap() == 5);
    assert!(receiver.try_recv().unwrap() == 6);
    assert!(receiver.try_recv() == Err(liten::sync::mpsc::RecvError::Empty));
  })
}

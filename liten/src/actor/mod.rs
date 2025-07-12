use std::future::Future;

use crate::{
  sync::request::{self, Requester},
  task::{self, TaskHandle},
};

pub enum ActorResult<V> {
  Result(V),
  Retry,
}

pub trait Actor<Message>
where
  Self: Send + Sized + 'static,
  Message: Send + 'static,
{
  type Output: Send;

  fn handle(
    self: &mut Self,
    payload: &Message,
  ) -> impl Future<Output = ActorResult<Self::Output>> + Send;

  fn start(self) -> ActorHandle<Message, Self::Output> {
    ActorRunner::start_with(self)
  }
}

pub struct ActorRunner;

impl ActorRunner {
  fn start_with<A, Message>(mut service: A) -> ActorHandle<Message, A::Output>
  where
    A: Actor<Message> + Send + 'static,
    Message: Send + 'static,
  {
    let (requester, mut responder) =
      request::channel::<Option<Message>, Option<A::Output>>();

    let task_handle = task::spawn(async move {
      'outer: loop {
        let Some((req, sender)) = responder.recv().await else {
          // Handle is dropped, this should exit.
          break;
        };
      }
    });
    //     let Some((req, sender)) = responder.recv().await else {
    //       // Handle is dropped, this should exit.
    //       break;
    //     };
    //
    //     let Some(message) = req else {
    //       // Shutdown signal
    //       let _ = sender.send(None);
    //       break;
    //     };
    //
    //     loop {
    //       let result = match service.handle(&message).await {
    //         ActorResult::Result(out) => out,
    //         ActorResult::Retry => continue,
    //       };
    //       if sender.send(Some(result)).is_err() {
    //         // Handle is dropped, this should exit.
    //         break 'outer;
    //       } else {
    //         break;
    //       };
    //     }
    // });

    ActorHandle::new(requester, task_handle)
  }
}

pub struct ActorHandle<Message, Res> {
  requester: Requester<Option<Message>, Option<Res>>,
  state: ActorHandleState,
}

struct ActorHandleState {
  handle: Option<TaskHandle<()>>,
}

impl ActorHandleState {
  fn with_handle(handle: TaskHandle<()>) -> Self {
    Self { handle: Some(handle) }
  }
}

impl<Message, Res> ActorHandle<Message, Res>
where
  Message: Send,
{
  fn new(
    requester: Requester<Option<Message>, Option<Res>>,
    handle: TaskHandle<()>,
  ) -> Self {
    Self { requester: requester, state: ActorHandleState::with_handle(handle) }
  }
  pub async fn send(&self, msg: Message) -> Option<Res> {
    self
      .requester
      .send(Some(msg))
      .await
      .expect("got back shutdown signal on send")
  }
  pub async fn stop(mut self) {
    let response = self.requester.send(None).await;

    assert!(response
      .expect("Channel cannot be dropped in the middle of calling stop")
      .is_none());

    self.state.handle.take().unwrap().await.expect("Actor body panicked");

    // Ignore None of return of requester because channel could be down which means handle is
    // dropped
  }
}

impl<Message, Res> Drop for ActorHandle<Message, Res> {
  fn drop(&mut self) {
    if self.state.handle.is_some() {
      let _ = self.requester.send_without_wait(None); // Don't care about dropping channel.
    }
  }
}

#[cfg(test)]
mod tests {

  use std::sync::atomic::{AtomicU8, Ordering};

  use crate::runtime::Runtime;

  use super::{Actor, ActorResult};

  static COUNT: AtomicU8 = AtomicU8::new(0);

  struct DemoActor;

  impl super::Actor<u8> for DemoActor {
    type Output = u8;
    async fn handle(
      self: &mut Self,
      input: &u8,
    ) -> super::ActorResult<Self::Output> {
      ActorResult::Result(COUNT.fetch_add(*input, Ordering::AcqRel) + *input)
    }
  }

  #[crate::internal_test]
  fn actors_work() {
    Runtime::single_threaded().block_on(async {
      let handle = DemoActor.start();

      handle.send(1).await;
      handle.send(1).await;
      handle.send(1).await;

      handle.stop().await;

      assert_eq!(COUNT.load(Ordering::Acquire,), 3);
    })
  }
}

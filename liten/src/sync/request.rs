use super::{mpmc, oneshot};

#[derive(Clone)]
pub struct Requester<Request, Response> {
  sender: mpmc::Sender<RequestPayload<Request, Response>>,
}

pub struct Responder<Request, Response> {
  receiver: mpmc::Receiver<RequestPayload<Request, Response>>,
}

pub struct RequestPayload<Req, Res> {
  respond_to: oneshot::Sender<Res>,
  value: Req,
}

pub fn channel<Req, Res>() -> (Requester<Req, Res>, Responder<Req, Res>) {
  let (sender, receiver) = mpmc::bounded(64);

  (Requester { sender }, Responder { receiver })
}

impl<Req, Res> Responder<Req, Res> {
  pub async fn recv(&mut self) -> Option<(Req, oneshot::Sender<Res>)> {
    let what = self.receiver.recv().await.ok()?;

    Some((what.value, what.respond_to))
  }
}

unsafe impl<Req: Send, Res: Send> Send for Responder<Req, Res> {}
unsafe impl<Req: Send, Res: Send> Send for Requester<Req, Res> {}

impl<Req, Res> Requester<Req, Res> {
  pub async fn send(&self, request: Req) -> Option<Res> {
    let (sender, receiver) = oneshot::channel();
    self
      .sender
      .try_send(RequestPayload { respond_to: sender, value: request })
      .ok()?;

    // TODO timeout
    receiver.await.ok()
  }

  pub fn send_without_wait(&self, request: Req) -> Option<()> {
    let (sender, _) = oneshot::channel();
    self
      .sender
      .try_send(RequestPayload { respond_to: sender, value: request })
      .ok()?;
    Some(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[crate::internal_test]
  fn request_response_roundtrip() {
    crate::runtime::Runtime::single_threaded().block_on(async {
      let (req, mut resp) = channel::<u32, u32>();

      // Spawn the responder in a separate task
      let responder_handle = crate::task::spawn(async move {
        let (val, sender) = resp.recv().await.unwrap();
        assert_eq!(val, 42);
        sender.send(val + 1).unwrap();
      });

      // Send request and await response
      let result = req.send(42).await;
      assert_eq!(result, Some(43));

      // Wait for responder to finish
      responder_handle.await.unwrap();
    });
  }

  #[test]
  fn drop_responder() {
    let (req, resp) = channel::<u32, u32>();
    drop(resp);
    let result = crate::future::block_on(req.send(1));
    assert_eq!(result, None);
  }
}

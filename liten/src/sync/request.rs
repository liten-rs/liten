use super::{mpsc, oneshot};

#[derive(Clone)]
pub struct Requester<Request, Response> {
  sender: mpsc::Sender<RequestPayload<Request, Response>>,
}

pub struct Responder<Request, Response> {
  receiver: mpsc::Receiver<RequestPayload<Request, Response>>,
}

pub struct RequestPayload<Req, Res> {
  respond_to: oneshot::Sender<Res>,
  value: Req,
}

pub fn channel<Req, Res>() -> (Requester<Req, Res>, Responder<Req, Res>) {
  let (sender, receiver) = mpsc::unbounded();

  (Requester { sender }, Responder { receiver })
}

impl<Req, Res> Responder<Req, Res> {
  pub async fn recv(&mut self) -> Option<(Req, oneshot::Sender<Res>)> {
    let what = self.receiver.recv().await.ok()?;

    Some((what.value, what.respond_to))
  }
}

impl<Req, Res> Requester<Req, Res> {
  pub async fn send(&self, request: Req) -> Option<Res> {
    let (sender, receiver) = oneshot::channel();
    self
      .sender
      .send(RequestPayload { respond_to: sender, value: request })
      .await
      .ok()?;

    // TODO timeout
    receiver.await.ok()
  }

  pub fn send_without_wait(&self, request: Req) -> Option<()> {
    let (sender, _) = oneshot::channel();
    self
      .sender
      .force_send(RequestPayload { respond_to: sender, value: request })
      .ok()?;
    Some(())
  }
}

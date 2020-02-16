//! ---------------------------------------------------
//! |                                                 |
//! |             WARNING! - proxy demo               |
//! | Not for production! Only for frontend debugging |
//! |     Allows payments without authentication!     |
//! |      TODO: Remove proxy mod and references      |
//! |                                                 |
//! ---------------------------------------------------
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use actix_web::web::{Bytes, Data};
use actix_web::{Error, HttpResponse, Responder};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{interval_at, Instant};

use super::Message;

pub async fn new_client(broadcaster: Data<Arc<Mutex<Broadcaster>>>) -> impl Responder {
    let rx = broadcaster.lock().unwrap().new_client();

    HttpResponse::Ok()
        .header("content-type", "text/event-stream")
        .no_chunking()
        .streaming(rx)
}

pub struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    pub fn create() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Broadcaster::new()))
    }

    fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    pub fn spawn_ping(me: Arc<Mutex<Self>>) {
        actix_rt::spawn(async move {
            let mut task = interval_at(Instant::now(), Duration::from_secs(10));
            while let Some(_) = task.next().await {
                me.lock().unwrap().remove_stale_clients();
            }
        })
    }

    fn remove_stale_clients(&mut self) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Bytes::from("data: ping\n\n"));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        tx.clone()
            .try_send(Bytes::from("data: connected\n\n"))
            .unwrap();

        self.clients.push(tx);
        Client(rx)
    }

    pub fn send_str(&self, msg: &str) {
        let msg = Bytes::from(["data: ", msg, "\n\n"].concat());

        for client in self.clients.iter() {
            client.clone().try_send(msg.clone()).unwrap_or(());
        }
    }

    pub fn send(&self, msg: Message) {
        let s = serde_json::to_string(&msg).unwrap();
        self.send_str(&s);
    }
}

// wrap Receiver in own type, with correct error type
struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

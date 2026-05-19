use std::{
    collections::HashMap,
    fmt,
    future::Future,
    sync::Arc,
};
use rmcp::{
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::Transport,
    RoleServer,
};
use serde::Deserialize;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
pub struct SseTransportError(String);

impl fmt::Display for SseTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SseTransportError {}

pub struct SseTransport {
    sse_tx: mpsc::Sender<ServerJsonRpcMessage>,
    client_rx: Arc<Mutex<Option<mpsc::Receiver<ClientJsonRpcMessage>>>>,
}

impl SseTransport {
    pub fn new() -> (
        Self,
        mpsc::Receiver<ServerJsonRpcMessage>,
        mpsc::Sender<ClientJsonRpcMessage>,
    ) {
        let (sse_tx, sse_rx) = mpsc::channel::<ServerJsonRpcMessage>(128);
        let (client_tx, client_rx) = mpsc::channel::<ClientJsonRpcMessage>(128);
        let transport = Self {
            sse_tx,
            client_rx: Arc::new(Mutex::new(Some(client_rx))),
        };
        (transport, sse_rx, client_tx)
    }
}

impl Transport<RoleServer> for SseTransport {
    type Error = SseTransportError;

    fn send(
        &mut self,
        item: ServerJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let tx = self.sse_tx.clone();
        async move {
            tx.send(item)
                .await
                .map_err(|e| SseTransportError(e.to_string()))
        }
    }

    async fn receive(&mut self) -> Option<ClientJsonRpcMessage> {
        let mut rx = self.client_rx.lock().await;
        if let Some(ref mut rx) = *rx {
            rx.recv().await
        } else {
            None
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        let mut rx = self.client_rx.lock().await;
        *rx = None;
        Ok(())
    }
}

pub type Sessions = Arc<Mutex<HashMap<String, mpsc::Sender<ClientJsonRpcMessage>>>>;

#[derive(Deserialize)]
pub struct MessageQuery {
    pub session_id: String,
}

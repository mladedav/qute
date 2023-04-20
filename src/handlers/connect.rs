use std::{
    future::{self, ready, Future},
    pin::Pin,
    sync::Arc,
};

use mqttbytes::v5::{ConnAck, Connect, Disconnect, Packet};
use tokio::sync::Notify;

pub(crate) struct ConnectHandler {
    state: ConnectState,
    ping_notify: Arc<Notify>,
}

#[derive(Debug)]
enum ConnectState {
    Disconnected,
    ConnectSent(Arc<Notify>),
    Connected,
}

impl ConnectHandler {
    pub(crate) fn new() -> ConnectHandler {
        Self {
            state: ConnectState::Disconnected,
            ping_notify: Arc::new(Notify::new()),
        }
    }

    pub fn connect(&mut self, _connect: &mut Connect) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let notify = Arc::new(Notify::new());
        self.state = ConnectState::ConnectSent(notify.clone());

        Box::pin(async move {
            notify.notified().await;
        })
    }

    pub fn connack(&mut self, _connack: ConnAck) -> Vec<Packet> {
        match &self.state {
            ConnectState::ConnectSent(notify) => {
                tracing::debug!("Notifying of CONNACK.");
                notify.notify_one();
                self.state = ConnectState::Connected;
            }
            _ => {
                tracing::error!(state = ?self.state, "CONNACK received when not expecting it.");
                panic!("Unexpected CONNACK.");
            }
        }
        self.state = ConnectState::Connected;
        Vec::new()
    }

    pub fn disconnect(&mut self, _disconnect: &mut Disconnect) -> Pin<Box<future::Ready<()>>> {
        self.state = ConnectState::Disconnected;
        Box::pin(ready(()))
    }

    pub(crate) fn ping(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        tracing::info!("Ping!");
        Box::pin({
            let notify = self.ping_notify.clone();
            async move {
                notify.notified().await;
            }
        })
    }

    pub(crate) fn pong(&self) -> Vec<Packet> {
        tracing::info!("Pong!");
        self.ping_notify.notify_waiters();
        Vec::new()
    }
}

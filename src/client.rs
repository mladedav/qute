use std::sync::Arc;

use mqttbytes::{
    v5::{Connect, Packet, Publish, Subscribe},
    QoS,
};
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

use crate::{
    connection::Connection,
    handlers::{
        connect::ConnectHandler,
        publish::{ReceivedPublishHandler, SentPublishHandler},
        subscribe::SubscribeHandler,
    },
    router::Router,
    subscribe::router::HandlerRouter,
};

#[derive(Clone)]
pub struct Client {
    router: Router<OwnedReadHalf, OwnedWriteHalf>,
}

impl Client {
    pub async fn connect(publish_router: HandlerRouter) -> Self {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        let connection = Arc::new(Connection::with_stream(stream));

        let router = Router {
            connection: connection.clone(),
            connect: Arc::new(Mutex::new(ConnectHandler::new())),
            sent_publish: Arc::new(Mutex::new(SentPublishHandler::new())),
            received_publish: Arc::new(Mutex::new(ReceivedPublishHandler::new(publish_router))),
            subscribe: Arc::new(Mutex::new(SubscribeHandler::new())),
        };

        tokio::spawn({
            let router = router.clone();
            async move {
                while let Some(packet) = connection.recv().await.unwrap() {
                    router.route_received(packet).await;
                }
                tracing::warn!("Connection closed.");
            }
        });

        let connect = Packet::Connect(Connect::new("qute"));
        router.route_sent(connect).await;

        Self { router }
    }

    pub async fn publish(&self, topic: &str, qos: QoS, payload: &[u8]) {
        let packet = Packet::Publish(Publish::new(topic, qos, payload));

        self.router.route_sent(packet).await;
    }

    pub async fn subscribe(&self, topic: &str) {
        let packet = Packet::Subscribe(Subscribe::new(topic, QoS::ExactlyOnce));

        self.router.route_sent(packet).await;
    }
}

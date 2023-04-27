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
    router: Arc<Mutex<Router<OwnedReadHalf, OwnedWriteHalf>>>,
}

impl Client {
    pub async fn connect(publish_router: HandlerRouter) -> Self {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        let connection = Arc::new(Connection::with_stream(stream));

        let router = Router {
            connection: connection.clone(),
            connect: ConnectHandler::new(),
            sent_publish: SentPublishHandler::new(),
            received_publish: ReceivedPublishHandler::new(publish_router),
            subscribe: SubscribeHandler::new(),
        };

        let router = Arc::new(Mutex::new(router));

        tokio::spawn({
            let connection = connection.clone();
            let router = router.clone();
            async move {
                while let Some(packet) = connection.recv().await.unwrap() {
                    router.lock().await.route_received(packet).await;
                }
                tracing::warn!("Connection closed.");
            }
        });

        let connect = Packet::Connect(Connect::new("qute"));
        let future = router.lock().await.route_sent(connect);
        // Do now hold the lock while waiting
        future.await;

        Self { router }
    }

    pub async fn publish(&self, topic: &str, qos: QoS, payload: &[u8]) {
        let packet = Packet::Publish(Publish::new(topic, qos, payload));

        let future = self.router.lock().await.route_sent(packet);
        // Do now hold the lock while waiting
        future.await;
    }

    pub async fn subscribe(&self, topic: &str) {
        let packet = Packet::Subscribe(Subscribe::new(topic, QoS::ExactlyOnce));

        let future = self.router.lock().await.route_sent(packet);
        // Do now hold the lock while waiting
        future.await;
    }
}

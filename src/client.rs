use std::{future::Future, pin::Pin, sync::Arc};

use mqttbytes::{
    v5::{Connect, Packet, Publish},
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
};

#[derive(Clone)]
pub struct Client {
    router: Arc<Mutex<Router<OwnedReadHalf, OwnedWriteHalf>>>,
}

impl Client {
    pub async fn connect() -> Self {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        let connection = Arc::new(Connection::with_stream(stream));

        let router = Router {
            connection: connection.clone(),
            connect: ConnectHandler::new(),
            sent_publish: SentPublishHandler::new(),
            received_publish: ReceivedPublishHandler::new(),
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
        // Split so we do not hold the lock
        let task = router.lock().await.route_sent(connect).await;
        task.await;

        Self { router }
    }

    pub async fn publish(
        &self,
        topic: &str,
        qos: QoS,
        payload: &[u8],
    ) -> Pin<Box<dyn Future<Output = ()>>> {
        let packet = Packet::Publish(Publish::new(topic, qos, payload));

        self.router.lock().await.route_sent(packet).await
    }
}

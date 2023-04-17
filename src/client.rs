use std::{sync::Arc, time::Duration};

use mqttbytes::v5::{Connect, Disconnect, Packet, Publish, Subscribe};
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
    _router: Arc<Mutex<Router<OwnedReadHalf, OwnedWriteHalf>>>,
}

impl Client {
    pub async fn connect() {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        let connection = Arc::new(Connection::with_stream(stream));

        let router = Router {
            connection: connection.clone(),
            connect: ConnectHandler::new(),
            sent_publish: SentPublishHandler::new(),
            received_publish: ReceivedPublishHandler::new(),
            subscribe: SubscribeHandler::new(),
            ping: crate::handlers::ping::PingHandler,
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
        let p1 = Packet::Publish(Publish::new("test", mqttbytes::QoS::AtMostOnce, *b"hello"));
        let p2 = Packet::Publish(Publish::new("test", mqttbytes::QoS::AtLeastOnce, *b"hello"));
        let p3 = Packet::Publish(Publish::new("test", mqttbytes::QoS::ExactlyOnce, *b"hello"));
        let s1 = Packet::Subscribe(Subscribe::new("test", mqttbytes::QoS::ExactlyOnce));

        router.lock().await.route_sent(connect).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        router.lock().await.route_sent(Packet::PingReq).await;
        router.lock().await.route_sent(p1).await;
        router.lock().await.route_sent(s1).await;
        router.lock().await.route_sent(p2).await;
        router.lock().await.route_sent(p3).await;
        router.lock().await.route_sent(Packet::PingReq).await;

        tokio::time::sleep(Duration::from_secs(1)).await;
        let d = Disconnect::new();
        connection.send(&Packet::Disconnect(d)).await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

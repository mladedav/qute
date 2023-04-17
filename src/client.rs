use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use mqttbytes::v5::{ConnAck, Connect, Disconnect, Packet, Publish, Subscribe};
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};
use tower::Service;

use crate::{
    connection::Connection,
    error::Error,
    handlers::{
        publish::{ReceivedPublishHandler, SentPublishHandler},
        subscribe::SubscribeHandler,
    },
    router::Router,
};

#[derive(Clone)]
pub struct Client {
    connection: Arc<Connection<OwnedReadHalf, OwnedWriteHalf>>,
}

impl Client {
    pub async fn connect() {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        let connection = Arc::new(Connection::with_stream(stream));

        let mut client = Self {
            connection: connection.clone(),
        };

        let connect = Connect::new("qute");

        let connack = client.call(connect).await.unwrap();
        tracing::info!(packet = ?connack, "Received CONNACK as a response to CONNECT.");

        let router = Router {
            connection: connection.clone(),
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

        let p1 = Packet::Publish(Publish::new("test", mqttbytes::QoS::AtMostOnce, *b"hello"));
        let p2 = Packet::Publish(Publish::new("test", mqttbytes::QoS::AtLeastOnce, *b"hello"));
        let p3 = Packet::Publish(Publish::new("test", mqttbytes::QoS::ExactlyOnce, *b"hello"));
        let s1 = Packet::Subscribe(Subscribe::new("test", mqttbytes::QoS::ExactlyOnce));

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

impl Service<Connect> for Client {
    type Response = ConnAck;

    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Connect) -> Self::Future {
        let connection = self.connection.clone();

        Box::pin(async move {
            connection.send(&Packet::Connect(req)).await.unwrap();

            match connection.recv().await {
                Err(err) => return Err(Error::MqttDeserialize(err)),
                Ok(None) => panic!("Connection closed"),
                Ok(Some(Packet::ConnAck(connack))) => return Ok(connack),
                Ok(Some(packet)) => panic!("Unexpected packet: {packet:?}"),
            }
        })
    }
}

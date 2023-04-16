use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use mqttbytes::v5::{ConnAck, Connect, Disconnect, Packet, Publish, Subscribe};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tower::Service;

use crate::{
    connection::Connection, error::Error, handlers::{publish::{SentPublishHandler, ReceivedPublishHandler}, subscribe::SubscribeHandler}, router::Router,
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

        let mut router = Router {
            connection: connection.clone(),
            sent_publish: SentPublishHandler::new(),
            received_publish: ReceivedPublishHandler::new(),
            subscribe: SubscribeHandler::new(),
            ping: crate::handlers::ping::PingHandler,
        };

        let mut p1 = Publish::new("test", mqttbytes::QoS::AtMostOnce, *b"hello");
        p1.pkid = 1;
        let p1 = Packet::Publish(p1);
        let mut p2 = Publish::new("test", mqttbytes::QoS::AtLeastOnce, *b"hello");
        p2.pkid = 2;
        let p2 = Packet::Publish(p2);
        let mut p3 = Publish::new("test", mqttbytes::QoS::ExactlyOnce, *b"hello");
        p3.pkid = 3;
        let p3 = Packet::Publish(p3);
        let mut s1 = Subscribe::new("test", mqttbytes::QoS::ExactlyOnce);
        s1.pkid = 4;
        let s1 = Packet::Subscribe(s1);

        router.route_sent(p1.clone()).await;
        router.route_sent(s1.clone()).await;
        router.route_sent(p2.clone()).await;
        router.route_sent(p3.clone()).await;
        // router.route_sent(Packet::PingReq).await;

        tokio::spawn({
            let connection = connection.clone();
            async move {
                while let Some(packet) = connection.recv().await.unwrap() {
                    router.route_received(packet).await;
                }
                tracing::warn!("Connection closed.");
            }
        });

        connection.send(&p1).await.unwrap();
        connection.send(&s1).await.unwrap();
        connection.send(&p2).await.unwrap();
        connection.send(&p3).await.unwrap();

        // connection.send(Packet::PingReq).await.unwrap();


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

use std::sync::Arc;

use mqttbytes::{
    v5::{Connect, Packet, Publish, Subscribe, SubscribeFilter},
    QoS,
};
use regex::Regex;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

use crate::{
    connection::Connection,
    router::{Publisher, Router, Subscriber},
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

        let single_wildcard = Regex::new(":[^/]*").unwrap();
        let multiple_wildcard = Regex::new(r"\*[^/]*$").unwrap();

        let to_subscribe: Vec<_> = publish_router
            .get_routes()
            .into_iter()
            .map(|route| {
                let replaced = single_wildcard.replace_all(&route, "+");
                let replaced = multiple_wildcard.replace(&replaced, "#");
                replaced.into_owned()
            })
            .collect();
        let router = Router::new(connection.clone(), publish_router);

        tokio::spawn({
            let router = router.clone();
            async move {
                while let Some(packet) = connection.recv().await.unwrap() {
                    tokio::spawn({
                        let router = router.clone();
                        async move {
                            router.route_received(packet).await;
                        }
                    });
                }
                tracing::warn!("Connection closed.");
            }
        });

        let connect = Packet::Connect(Connect::new("qute"));
        router.route_sent(connect).await;

        let subscribe = Packet::Subscribe(Subscribe::new_many(
            to_subscribe
                .into_iter()
                .map(|topic| SubscribeFilter::new(topic, QoS::ExactlyOnce)),
        ));
        router.route_sent(subscribe).await;

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

#[derive(Clone)]
pub struct ClientState {
    pub(crate) publisher: Publisher,
    pub(crate) subscriber: Subscriber,
}

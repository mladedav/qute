use std::sync::Arc;

use mqttbytes::{
    v5::{Connect, Disconnect, Packet, Publish, Subscribe, SubscribeFilter},
    QoS,
};
use regex::Regex;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::task::TaskTracker;

use crate::{
    connection::Connection,
    router::{Publisher, Router, Subscriber},
    subscribe::router::HandlerRouter,
};

pub use builder::ClientBuilder;

mod builder;

#[derive(Clone)]
pub struct Client {
    router: Router<OwnedReadHalf, OwnedWriteHalf>,
    tracker: TaskTracker,
}

impl Client {
    async fn connect(
        tcp_stream: TcpStream,
        publish_router: HandlerRouter,
        connect: Connect,
    ) -> Self {
        let connection = Arc::new(Connection::with_stream(tcp_stream));

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
        let tracker = TaskTracker::new();

        tokio::spawn({
            let router = router.clone();
            let tracker = tracker.clone();
            async move {
                while let Some(packet) = connection.recv().await.unwrap() {
                    tracker.spawn({
                        let router = router.clone();
                        async move {
                            router.route_received(packet).await;
                        }
                    });
                }
                tracing::debug!("Connection closed.");

                tracker.close();
                tracker.wait().await;

                tracing::debug!("All tasks processed.");
            }
        });

        let connect = Packet::Connect(connect);
        router.route_sent(connect).await;

        let subscribe = Packet::Subscribe(Subscribe::new_many(
            to_subscribe
                .into_iter()
                .map(|topic| SubscribeFilter::new(topic, QoS::ExactlyOnce)),
        ));
        router.route_sent(subscribe).await;

        Self { router, tracker }
    }

    pub async fn publish(&self, topic: &str, qos: QoS, payload: &[u8]) {
        let packet = Packet::Publish(Publish::new(topic, qos, payload));

        self.router.route_sent(packet).await;
    }

    pub async fn subscribe(&self, topic: &str) {
        let packet = Packet::Subscribe(Subscribe::new(topic, QoS::ExactlyOnce));

        self.router.route_sent(packet).await;
    }

    pub async fn shutdown(mut self) {
        let packet = Packet::Disconnect(Disconnect::new());
        self.router.route_sent(packet).await;
        self.router.shutdown().await;
        self.tracker.wait().await;
    }
}

#[derive(Clone)]
pub struct ClientState {
    pub(crate) publisher: Publisher,
    pub(crate) subscriber: Subscriber,
}

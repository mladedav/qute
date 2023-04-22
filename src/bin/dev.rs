use std::convert::Infallible;

use mqttbytes::{v5::Publish, QoS};
use qute::{Client, FromPublish, HandlerRouter, State, Topic};
use tokio::task::yield_now;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("qute=debug,info")
        .finish()
        .init();

    let mut router = HandlerRouter::new();

    router.add(
        String::from("test"),
        |_publish: Publish, State(state): State<&str>| {
            tracing::warn!("Test handler!");
            tracing::warn!(?state, "Now with state!");
        },
        "This is the state.",
    );

    router.add(String::from("foo/bar"), foobar, ());

    let client = Client::connect(router).await;
    client.subscribe("test").await.await;
    client.subscribe("foo/bar").await.await;
    client
        .publish("test", QoS::AtMostOnce, b"hello")
        .await
        .await;
    client
        .publish("test", QoS::AtLeastOnce, b"hello world")
        .await
        .await;
    client
        .publish("test", QoS::ExactlyOnce, b"hello complicated world")
        .await
        .await;
    client
        .publish("foo/bar", QoS::AtMostOnce, b"hello")
        .await
        .await;
    client.publish("foo", QoS::AtMostOnce, b"hello").await.await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

async fn foobar(
    _publish: Publish,
    qos: QoS,
    Topic(topic): Topic,
    custom: Custom,
    _p5: Publish,
    _p6: Publish,
    _p7: Publish,
) {
    yield_now().await;
    tracing::warn!(
        ?qos,
        ?topic,
        ?custom,
        "Async FOOBAR handler with quite a few extractors!"
    );
}

#[derive(Debug)]
struct Custom(String);

impl<S> FromPublish<S> for Custom {
    type Rejection = Infallible;

    fn from_publish(_publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Custom(String::from("My implementation of FromPublish.")))
    }
}

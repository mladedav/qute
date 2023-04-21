use mqttbytes::{v5::Publish, QoS};
use qute::{Client, HandlerRouter};
use tokio::{task::yield_now};
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("qute=debug,info")
        .finish()
        .init();

    let mut router = HandlerRouter::new();

    router
        .add(String::from("test"), |_publish: Publish| {
            tracing::warn!("Test handler!");
        });

    router.add(String::from("foo/bar"), foobar);

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

async fn foobar(_publish: Publish) {
    yield_now().await;
    tracing::warn!("Async FOOBAR handler!");
}

use std::convert::Infallible;

use mqttbytes::{v5::Publish, QoS};
use qute::{Client, Extractable, FromState, HandlerRouterBuilder, State, Topic};
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

    let mut router = HandlerRouterBuilder::<String>::new();

    router.add("test", |_publish: Publish, State(state): State<String>| {
        tracing::warn!("Test handler!");
        tracing::warn!(?state, "Now with state!");
    });
    let mut router = router.with_state(String::from("This is the state."));

    router.add("foo/bar", foobar);
    let router = router.with_state(OuterState(InnerState)).build();

    let client = Client::connect(router).await;
    client.subscribe("test").await;
    client.subscribe("foo/bar").await;
    client.publish("test", QoS::AtMostOnce, b"hello").await;
    client
        .publish("test", QoS::AtLeastOnce, b"hello world")
        .await;
    client
        .publish("test", QoS::ExactlyOnce, b"hello complicated world")
        .await;
    client.publish("foo/bar", QoS::AtMostOnce, b"hello").await;
    client.publish("foo", QoS::AtMostOnce, b"hello").await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

#[derive(Clone, Debug)]
struct InnerState;
#[derive(Clone, Debug)]
struct OuterState(InnerState);

impl FromState<OuterState> for InnerState {
    fn from_state(state: &OuterState) -> Self {
        state.0.clone()
    }
}

async fn foobar(
    _publish: Publish,
    qos: QoS,
    Topic(topic): Topic,
    custom: Custom,
    State(outer): State<OuterState>,
    State(inner): State<InnerState>,
) {
    yield_now().await;
    tracing::warn!(
        ?qos,
        ?topic,
        ?custom,
        ?outer,
        ?inner,
        "Async FOOBAR handler with quite a few extractors!"
    );
}

#[derive(Debug)]
struct Custom(String);

impl<S> Extractable<S> for Custom {
    type Rejection = Infallible;

    fn extract(_publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Custom(String::from("My implementation of FromPublish.")))
    }
}

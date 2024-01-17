use std::convert::Infallible;

use mqttbytes::{v5::Publish, QoS};
use qute::{
    ClientBuilder, ClientState, Extractable, FromState, HandlerRouterBuilder, Publisher, State,
    Subscriber, Topic,
};
use tokio::task::yield_now;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("qute=debug,info")
        .finish()
        .init();

    let mut router = HandlerRouterBuilder::<String>::new();

    router.add("test", |publish: Publish, State(state): State<String>| {
        tracing::warn!("Test handler!");
        tracing::warn!(?publish, ?state, "Now with state!");
    });
    let mut router = router.with_state(String::from("This is the state."));

    router.add("foo/:bar", foobar);
    router.add("foobar/*rest", foobar);
    router.add(":foo/bar", foobar);
    let mut router = router.with_state(OuterState(InnerState));
    router.add("callback", |publish: Publish| {
        tracing::warn!(?publish, "Callback handler!");
    });
    let router = router.build();

    let client = ClientBuilder::new("127.0.0.1:1883").build(router).await;

    client.publish("test", QoS::AtMostOnce, b"hello").await;
    client
        .publish("test", QoS::AtLeastOnce, b"hello world")
        .await;
    client
        .publish("test", QoS::ExactlyOnce, b"hello complicated world")
        .await;
    client.publish("foo", QoS::AtLeastOnce, b"hello").await;
    client.publish("foo/bar", QoS::AtLeastOnce, b"hello").await;
    client
        .publish("foo/bar/baz", QoS::AtLeastOnce, b"hello")
        .await;
    client.publish("foo/foo", QoS::AtLeastOnce, b"hello").await;
    client.publish("bar/bar", QoS::AtLeastOnce, b"hello").await;
    client.publish("foobar", QoS::AtLeastOnce, b"hello").await;
    client
        .publish("foobar/baz", QoS::AtLeastOnce, b"hello")
        .await;
    client
        .publish("foobar/baz/bax", QoS::AtLeastOnce, b"hello")
        .await;

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

#[allow(clippy::too_many_arguments)]
async fn foobar(
    _publish: Publish,
    qos: QoS,
    Topic(topic): Topic,
    custom: Custom,
    State(outer): State<OuterState>,
    State(inner): State<InnerState>,
    publisher: Publisher,
    _subscriber: Subscriber,
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

    tracing::info!("Subscribing to new topic.");
    // subscriber.subscribe("callback").await;
    tracing::info!("Publishing stuff from handler.");
    publisher
        .publish("callback", QoS::AtLeastOnce, b"Real callback!")
        .await;
    tracing::info!("Stuff from handler published.");
}

#[derive(Debug)]
struct Custom(String);

impl<S> Extractable<S> for Custom {
    type Rejection = Infallible;

    fn extract(
        _publish: &Publish,
        _state: &S,
        _client: &ClientState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Custom(String::from("My implementation of Extractable.")))
    }
}

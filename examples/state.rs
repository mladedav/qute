use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use mqttbytes::{v5::Publish, QoS};
use qute::{ClientBuilder, HandlerRouterBuilder, State};

#[tokio::main]
async fn main() {
    let mut handlers = HandlerRouterBuilder::<String>::new();

    handlers.add("test", |publish: Publish, State(state): State<String>| {
        println!("Handler with string state `{state}` received message: {publish:?}.");
    });
    let mut handlers = handlers.with_state(String::from("This is the state."));

    handlers.add("count", |State(state): State<Arc<AtomicU32>>| {
        let count = state.fetch_add(1, Ordering::Relaxed);
        println!("This handler has been called {count} times.");
    });
    let handlers = handlers.with_state(Arc::new(AtomicU32::new(0))).build();

    let client = ClientBuilder::new("127.0.0.1:1883").build(handlers).await;

    client.publish("count", QoS::AtMostOnce, b"hello").await;
    client.publish("test", QoS::AtMostOnce, b"hello").await;
    client
        .publish(
            "count",
            QoS::AtMostOnce,
            b"This will not be printed anyway.",
        )
        .await;

    client.shutdown().await;
}

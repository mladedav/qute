use std::convert::Infallible;

use mqttbytes::{v5::Publish, QoS};
use qute::{ClientBuilder, ClientState, Extractable, HandlerRouterBuilder};

#[tokio::main]
async fn main() {
    let mut handlers = HandlerRouterBuilder::new();

    handlers.add("test", |custom: Custom| {
        println!("Handler called with a custom argument `{custom:?}`.");
    });

    let handlers = handlers.build();

    let client = ClientBuilder::new("127.0.0.1:1883").build(handlers).await;

    client.publish("test", QoS::AtMostOnce, b"hello").await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
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

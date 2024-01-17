use mqttbytes::QoS;
use qute::{ClientBuilder, HandlerRouterBuilder};

#[tokio::main]
async fn main() {
    let mut handlers = HandlerRouterBuilder::new();

    handlers.add("test", || {
        println!("Test handler!");
    });

    let handlers = handlers.build();

    let client = ClientBuilder::new("127.0.0.1:1883").build(handlers).await;

    // Send a test message that the client then handles.
    client.publish("test", QoS::AtMostOnce, b"hello").await;
}

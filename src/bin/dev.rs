use mqttbytes::QoS;
use qute::Client;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("qute=debug,info")
        .finish()
        .init();

    let client = Client::connect().await;
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

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

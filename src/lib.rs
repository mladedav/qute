mod client;
mod connection;
mod error;
mod handlers;
mod router;

pub use client::Client;
use mqttbytes::v5::Publish;
use tower::Service;

pub trait MqttService {
    type Future;

    fn process(&mut self, publish: Publish) -> Self::Future;
}

impl<T> MqttService for T
where
    T: Service<Publish>,
{
    type Future = T::Future;

    fn process(&mut self, publish: Publish) -> Self::Future {
        self.call(publish)
    }
}

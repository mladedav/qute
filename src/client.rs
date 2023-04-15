use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use mqttbytes::v5::{ConnAck, Connect, Packet};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tower::Service;

use crate::{connection::Connection, error::Error};

#[derive(Clone)]
pub struct Client {
    connection: Arc<Connection<OwnedReadHalf, OwnedWriteHalf>>,
}

impl Client {
    pub async fn connect() {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        let connection = Arc::new(Connection::with_stream(stream));

        let mut client = Self { connection };

        let connect = Connect::new("qute");

        let connack = client.call(connect).await.unwrap();
        tracing::info!(packet = ?connack, "Received CONNACK as a response to CONNECT.");
    }
}

impl Service<Connect> for Client {
    type Response = ConnAck;

    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Connect) -> Self::Future {
        let connection = self.connection.clone();

        Box::pin(async move {
            connection.send(Packet::Connect(req)).await.unwrap();

            match connection.recv().await {
                Err(err) => return Err(Error::MqttDeserialize(err)),
                Ok(None) => panic!("Connection closed"),
                Ok(Some(Packet::ConnAck(connack))) => return Ok(connack),
                Ok(Some(packet)) => panic!("Unexpected packet: {packet:?}"),
            }
        })
    }
}

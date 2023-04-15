use std::{future::Future, pin::Pin, task::{Poll, Context}, sync::Arc};

use bytes::{Buf, BytesMut};
use mqttbytes::{v5::{Connect, ConnAck, Packet}};
use thiserror::Error as ThisError;
use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncReadExt, AsyncRead, AsyncWrite}, sync::RwLock};
use tower::Service;

const MAX_SIZE: usize = 1024;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Cannot deserialize MQTT message: {0:?}")]
    MqttDeserialize(mqttbytes::Error),
}

#[derive(Clone)]
pub struct Client<T> {
    stream: T,
}

impl Client<Arc<RwLock<TcpStream>>> {
    // pub fn new(stream: TcpStream) -> Self {
    // }

    pub async fn connect() {
        let stream = TcpStream::connect("127.0.0.1:1883").await.unwrap();
        // let stream = TcpStream::connect("10.200.200.3:1883").await.unwrap();
        let stream = Arc::new(RwLock::new(stream));

        let mut client = Self {
            stream,
        };

        let connect = Connect::new("qute");

        client.call(connect).await.unwrap();
    }
}

impl<T> Service<Connect> for Client<Arc<RwLock<T>>>
where
T: AsyncRead + AsyncWrite + Unpin + 'static,
{
    type Response = ConnAck;

    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Connect) -> Self::Future {
        let socket = self.stream.clone();
        Box::pin(async move {
            let mut buf = BytesMut::new();
            req.write(&mut buf).unwrap();
            let mut buf = buf.freeze();
            while buf.has_remaining() {
                socket.write().await.write_buf(&mut buf).await.unwrap();
            }
            let mut response = BytesMut::new();
            loop {
                if socket.write().await.read_buf(&mut response).await.unwrap() == 0 {
                    tracing::debug!(len = response.len(), buf = ?response, "Unexpected EOF.");
                    panic!("Unexpected EOF.");
                }

                match mqttbytes::v5::read(&mut response, MAX_SIZE) {
                    Err(mqttbytes::Error::InsufficientBytes(len)) => {
                        tracing::debug!(required = len, "Insufficient bytes, more are required.");
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, "Unable to read packet.");
                        return Err(Error::MqttDeserialize(e));
                    }
                    Ok(p) => {
                        println!("Got packet {p:?}");
                        if let Packet::ConnAck(connack) = p {
                            return Ok(connack);
                        }
                    }
                }
            }
        })
    }
}

// impl Service<Packet> for Client {
//     type Response = ();

//     type Error = mqttbytes::Error;

//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

//     fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
//         todo!()
//     }

//     fn call(&mut self, req: Packet) -> Self::Future {
//         todo!()
//     }
// }

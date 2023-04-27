use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use mqttbytes::v5::Packet;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    connection::{self, Connection},
    handlers::{
        connect::ConnectHandler,
        publish::{ReceivedPublishHandler, SentPublishHandler},
        subscribe::SubscribeHandler,
    },
};

pub(crate) struct Router<R, W> {
    pub connection: Arc<Connection<R, W>>,

    pub connect: ConnectHandler,
    pub sent_publish: SentPublishHandler,
    pub received_publish: ReceivedPublishHandler,
    pub subscribe: SubscribeHandler,
}

impl<R, W> Router<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin + Send +'static,
{
    pub async fn route_received(&mut self, packet: Packet) {
        tracing::debug!(?packet, "Routing received packet.");

        let responses = match packet {
            Packet::Connect(_) => unreachable!("Client cannot receive connect."),
            Packet::ConnAck(packet) => self.connect.connack(packet),
            Packet::Disconnect(_) => unreachable!("Client cannot receive disconnect."),

            Packet::PingReq => unreachable!("Client cannot receive ping request."),
            Packet::PingResp => self.connect.pong(),

            Packet::Publish(packet) => self.received_publish.publish(packet).await,
            Packet::PubAck(packet) => self.sent_publish.puback(packet),
            Packet::PubRec(packet) => self.sent_publish.pubrec(packet),
            Packet::PubRel(packet) => self.received_publish.pubrel(packet),
            Packet::PubComp(packet) => self.sent_publish.pubcomp(packet),

            Packet::Subscribe(_) => unreachable!("Client cannot receive subscribe."),
            Packet::SubAck(packet) => self.subscribe.suback(packet),

            Packet::Unsubscribe(_) => unreachable!("Client cannot receive unsubscribe."),
            Packet::UnsubAck(packet) => self.subscribe.unsuback(packet),
        };

        for response in responses {
            self.route_sent(response).await;
        }
    }

    pub fn route_sent(&mut self, mut packet: Packet) -> SendFuture<R, W> {
        tracing::debug!(?packet, "Routing sent packet.");

        let future = self.prepare_packet(&mut packet);
        SendFuture::new(self.connection.clone(), packet, future)
    }

    // This function (and every function in the match inside) both mutates the packet before it can be sent (e.g. adds packet ID to PUBLISH packets) and provides a future that resolves after the packet has been resolved (e.g. PUBLISH wih QoS 1 has been acknowledged).
    pub fn prepare_packet(
        &mut self,
        packet: &mut Packet,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        match packet {
            Packet::Connect(packet) => self.connect.connect(packet),
            Packet::ConnAck(_) => unreachable!("Client cannot send connect acknowledgement."),
            Packet::Disconnect(packet) => self.connect.disconnect(packet),

            Packet::PingReq => self.connect.ping(),
            Packet::PingResp => unreachable!("Client cannot send ping response."),

            Packet::Publish(packet) => self.sent_publish.publish(packet),
            Packet::PubAck(packet) => self.received_publish.puback(packet),
            Packet::PubRec(packet) => self.received_publish.pubrec(packet),
            Packet::PubRel(packet) => self.sent_publish.pubrel(packet),
            Packet::PubComp(packet) => self.received_publish.pubcomp(packet),

            Packet::Subscribe(packet) => self.subscribe.subscribe(packet),
            Packet::SubAck(_) => unreachable!("Client cannot send subscribe acknowledgement."),

            Packet::Unsubscribe(packet) => self.subscribe.unsubscribe(packet),
            Packet::UnsubAck(_) => unreachable!("Client cannot send unsubscribe acknowledgement."),
        }
    }
}

enum SendFutureState<W> {
    New,
    Sending(connection::SendFuture<W>),
    WaitingForResult,
}

// unsafe impl<'a, R, W> Send for SendFuture<'a, R, W> {}

pub struct SendFuture<R, W> {
    state: SendFutureState<W>,
    connection: Arc<Connection<R, W>>,
    packet: Packet,
    process_future: Pin<Box<dyn Future<Output = ()> + Send>>,
}
impl<R, W> SendFuture<R, W> {
    fn new(
        connection: Arc<Connection<R, W>>,
        packet: Packet,
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> SendFuture<R, W> {
        SendFuture {
            state: SendFutureState::New,
            connection,
            packet,
            process_future: future,
        }
    }
}

impl<R, W> Future for SendFuture<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                SendFutureState::New => {
                    let connection = this.connection.clone();
                    let packet = this.packet.clone();
                    let future = connection.send(&packet).unwrap();
                    this.state = SendFutureState::Sending(future);
                }
                SendFutureState::Sending(future) => {
                    ready!(Pin::new(future).poll(cx));
                    this.state = SendFutureState::WaitingForResult;
                }
                SendFutureState::WaitingForResult => {
                    return this.process_future.as_mut().poll(cx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

    use super::*;

    fn is_send<T: Send>() {}

    #[allow(dead_code)]
    fn assert_send() {
        is_send::<SendFuture<OwnedReadHalf, OwnedWriteHalf>>();
        is_send::<Router<OwnedReadHalf, OwnedWriteHalf>>();
    }
}

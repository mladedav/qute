use std::{future::Future, pin::Pin, sync::Arc};

use mqttbytes::v5::Packet;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};

use crate::{
    connection::Connection,
    handlers::{
        connect::ConnectHandler,
        publish::{ReceivedPublishHandler, SentPublishHandler},
        subscribe::SubscribeHandler,
    },
};

pub(crate) struct Router<R, W> {
    pub connection: Arc<Connection<R, W>>,

    pub connect: Arc<Mutex<ConnectHandler>>,
    pub sent_publish: Arc<Mutex<SentPublishHandler>>,
    pub received_publish: Arc<Mutex<ReceivedPublishHandler>>,
    pub subscribe: Arc<Mutex<SubscribeHandler>>,
}

impl<R, W> Clone for Router<R, W> {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            connect: self.connect.clone(),
            sent_publish: self.sent_publish.clone(),
            received_publish: self.received_publish.clone(),
            subscribe: self.subscribe.clone(),
        }
    }
}

impl<R, W> Router<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin + Send + 'static,
{
    pub async fn route_received(&self, packet: Packet) {
        tracing::debug!(?packet, "Routing received packet.");

        let responses = match packet {
            Packet::Connect(_) => unreachable!("Client cannot receive connect."),
            Packet::ConnAck(packet) => self.connect.lock().await.connack(packet),
            Packet::Disconnect(_) => unreachable!("Client cannot receive disconnect."),

            Packet::PingReq => unreachable!("Client cannot receive ping request."),
            Packet::PingResp => self.connect.lock().await.pong(),

            Packet::Publish(packet) => self.received_publish.lock().await.publish(packet).await,
            Packet::PubAck(packet) => self.sent_publish.lock().await.puback(packet),
            Packet::PubRec(packet) => self.sent_publish.lock().await.pubrec(packet),
            Packet::PubRel(packet) => self.received_publish.lock().await.pubrel(packet),
            Packet::PubComp(packet) => self.sent_publish.lock().await.pubcomp(packet),

            Packet::Subscribe(_) => unreachable!("Client cannot receive subscribe."),
            Packet::SubAck(packet) => self.subscribe.lock().await.suback(packet),

            Packet::Unsubscribe(_) => unreachable!("Client cannot receive unsubscribe."),
            Packet::UnsubAck(packet) => self.subscribe.lock().await.unsuback(packet),
        };

        for response in responses {
            self.route_sent(response).await;
        }
    }

    pub async fn route_sent(&self, mut packet: Packet) {
        tracing::debug!(?packet, "Routing sent packet.");

        let future = self.prepare_packet(&mut packet).await;
        self.connection.send(&packet).unwrap().await;
        future.await;
    }

    // This function (and every function in the match inside) both mutates the packet before it can be sent (e.g. adds packet ID to PUBLISH packets) and provides a future that resolves after the packet has been resolved (e.g. PUBLISH wih QoS 1 has been acknowledged).
    pub async fn prepare_packet(
        &self,
        packet: &mut Packet,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        match packet {
            Packet::Connect(packet) => self.connect.lock().await.connect(packet),
            Packet::ConnAck(_) => unreachable!("Client cannot send connect acknowledgement."),
            Packet::Disconnect(packet) => self.connect.lock().await.disconnect(packet),

            Packet::PingReq => self.connect.lock().await.ping(),
            Packet::PingResp => unreachable!("Client cannot send ping response."),

            Packet::Publish(packet) => self.sent_publish.lock().await.publish(packet),
            Packet::PubAck(packet) => self.received_publish.lock().await.puback(packet),
            Packet::PubRec(packet) => self.received_publish.lock().await.pubrec(packet),
            Packet::PubRel(packet) => self.sent_publish.lock().await.pubrel(packet),
            Packet::PubComp(packet) => self.received_publish.lock().await.pubcomp(packet),

            Packet::Subscribe(packet) => self.subscribe.lock().await.subscribe(packet),
            Packet::SubAck(_) => unreachable!("Client cannot send subscribe acknowledgement."),

            Packet::Unsubscribe(packet) => self.subscribe.lock().await.unsubscribe(packet),
            Packet::UnsubAck(_) => unreachable!("Client cannot send unsubscribe acknowledgement."),
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
        is_send::<Router<OwnedReadHalf, OwnedWriteHalf>>();
    }
}

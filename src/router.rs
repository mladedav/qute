use std::sync::Arc;

use mqttbytes::v5::Packet;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{handlers::{ping::PingHandler, publish::{SentPublishHandler, ReceivedPublishHandler}, subscribe::SubscribeHandler}, connection::Connection};

pub(crate) struct Router<R, W> {
    pub connection: Arc<Connection<R, W>>,

    pub ping: PingHandler,
    pub sent_publish: SentPublishHandler,
    pub received_publish: ReceivedPublishHandler,
    pub subscribe: SubscribeHandler,
}

impl<R, W> Router<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub async fn route_received(&mut self, packet: Packet) {
        tracing::debug!(?packet, "Routing received packet.");

        let responses = match packet {
            Packet::Connect(packet) => unreachable!("Client cannot receive connect."),
            Packet::ConnAck(packet) => todo!(),
            Packet::Disconnect(packet) => unreachable!("Client cannot receive disconnect."),

            Packet::Publish(packet) => self.received_publish.publish(packet),
            Packet::PubAck(packet) => self.sent_publish.puback(packet),
            Packet::PubRec(packet) => self.sent_publish.pubrec(packet),
            Packet::PubRel(packet) => self.received_publish.pubrel(packet),
            Packet::PubComp(packet) => self.sent_publish.pubcomp(packet),

            Packet::Subscribe(packet) => unreachable!("Client cannot receive subscribe."),
            Packet::SubAck(packet) => self.subscribe.suback(packet),

            Packet::Unsubscribe(packet) => unreachable!("Client cannot receive unsubscribe."),
            Packet::UnsubAck(packet) => self.subscribe.unsuback(packet),

            Packet::PingReq => unreachable!("Client cannot receive ping request."),
            Packet::PingResp => self.ping.pong(),
        };

        for response in responses {
            self.connection.send(&response).await.unwrap();
            self.route_sent(response).await;
        }
    }

    pub async fn route_sent(&mut self, packet: Packet) {
        tracing::debug!(?packet, "Routing sent packet.");

        match packet {
            Packet::Connect(packet) => todo!(),
            Packet::ConnAck(packet) => unreachable!("Client cannot send connect acknowledgement."),
            Packet::Disconnect(packet) => todo!(),

            Packet::Publish(packet) => self.sent_publish.publish(packet),
            Packet::PubAck(packet) => self.received_publish.puback(packet),
            Packet::PubRec(packet) => self.received_publish.pubrec(packet),
            Packet::PubRel(packet) => self.sent_publish.pubrel(packet),
            Packet::PubComp(packet) => self.received_publish.pubcomp(packet),

            Packet::Subscribe(packet) => self.subscribe.subscribe(packet),
            Packet::SubAck(packet) => unreachable!("Client cannot send subscribe acknowledgement."),

            Packet::Unsubscribe(packet) => self.subscribe.unsubscribe(packet),
            Packet::UnsubAck(packet) => unreachable!("Client cannot send unsubscribe acknowledgement."),

            Packet::PingReq => self.ping.ping(),
            Packet::PingResp => unreachable!("Client cannot send ping response."),
        };
    }
}

use mqttbytes::v5::Packet;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::handlers::{ping::PingHandler, publish::PublishHandler};

pub(crate) struct Router<R, W> {
    pub ping: PingHandler,
    pub publish: PublishHandler<R, W>,
}

impl<R, W> Router<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub async fn route_received(&mut self, packet: Packet) {
        tracing::debug!(?packet, "Routing received packet.");

        match packet {
            Packet::Connect(packet) => unreachable!("Client cannot receive connect."),
            Packet::ConnAck(packet) => todo!(),
            Packet::Disconnect(packet) => unreachable!("Client cannot receive disconnect."),

            Packet::Publish(packet) => self.publish.recv_publish(packet).await,
            Packet::PubAck(packet) => self.publish.puback(packet),
            Packet::PubRec(packet) => self.publish.pubrec(packet).await,
            Packet::PubRel(packet) => self.publish.pubrel(packet).await,
            Packet::PubComp(packet) => self.publish.pubcomp(packet),

            Packet::Subscribe(packet) => todo!(),
            Packet::SubAck(packet) => todo!(),

            Packet::Unsubscribe(packet) => todo!(),
            Packet::UnsubAck(packet) => todo!(),

            Packet::PingReq => unreachable!("Client cannot receive ping request."),
            Packet::PingResp => self.ping.pong(),
        }
    }

    pub async fn route_sent(&mut self, packet: Packet) {
        tracing::debug!(?packet, "Routing sent packet.");

        match packet {
            Packet::Connect(packet) => todo!(),
            Packet::ConnAck(packet) => unreachable!("Client cannot send connect acknowledgement."),
            Packet::Disconnect(packet) => todo!(),

            Packet::Publish(packet) => self.publish.send_publish(packet),
            Packet::PubAck(packet) => todo!(),
            Packet::PubRec(packet) => todo!(),
            Packet::PubRel(packet) => todo!(),
            Packet::PubComp(packet) => todo!(),

            Packet::Subscribe(packet) => todo!(),
            Packet::SubAck(packet) => todo!(),

            Packet::Unsubscribe(packet) => todo!(),
            Packet::UnsubAck(packet) => todo!(),

            Packet::PingReq => self.ping.ping(),
            Packet::PingResp => todo!(),
        }
    }
}

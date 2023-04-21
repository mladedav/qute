use std::{convert::Infallible, future::Future, pin::Pin, sync::Arc};

use mqttbytes::v5::{Packet, Publish};
use tokio::{
    io::{AsyncRead, AsyncWrite},
};
use tower::{util::BoxCloneService, ServiceExt};
use tower::Service;

use crate::{
    connection::Connection,
    handlers::{
        connect::ConnectHandler,
        publish::{ReceivedPublishHandler, SentPublishHandler},
        subscribe::SubscribeHandler,
    },
    Handler,
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
    W: AsyncWrite + Unpin,
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
            self.connection.send(&response).await.unwrap();
            self.route_sent(response).await;
        }
    }

    pub async fn route_sent(
        &mut self,
        mut packet: Packet,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        tracing::debug!(?packet, "Routing sent packet.");

        let future = match &mut packet {
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
        };

        self.connection.send(&packet).await.unwrap();

        future
    }
}

pub struct HandlerRouter {
    inner: matchit::Router<BoxCloneService<Publish, (), Infallible>>,

}

impl HandlerRouter {
    pub fn new() -> Self {
        Self {
            inner: matchit::Router::new(),
        }
    }

    pub fn add<const ASYNC: bool>(&mut self, route: String, handler: impl Handler<ASYNC>) {
        let handler = handler.with_state(());
        let handler = BoxCloneService::new(handler);
        self.inner.insert(route, handler).unwrap();
    }

    pub async fn handle(&mut self, publish: Publish) {
        if let Ok(router_match) = self.inner.at_mut(&publish.topic) {
            let service = router_match.value.ready().await.expect("Error type is Infallible.");
            service.call(publish.clone()).await.expect("Error type is Infallible.");
        } else {
            tracing::debug!(topic = %publish.topic, "No matching route found.");
        }
    }
}

impl Default for HandlerRouter {
    fn default() -> Self {
        Self::new()
    }
}

use mqttbytes::v5::Packet;
use tower::Service;

use crate::error::Error;

pub(super) mod ping;
pub(super) mod publish;
pub(super) mod subscribe;

trait OutgoingMqttService<Request> {
}

trait IncomingHandler<Request> {
    fn handle(packet: Request) -> Vec<Packet>;
}

trait OutgoingHandler<Request> {
    fn handle(packet: &mut Request);
}

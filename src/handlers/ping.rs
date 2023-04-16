use mqttbytes::v5::Packet;

pub(crate) struct PingHandler;

impl PingHandler {
    pub(crate) fn pong(&self) -> Vec<Packet> {
        tracing::info!("Pong!");
        Vec::new()
    }

    pub(crate) fn ping(&self) -> Vec<Packet> {
        tracing::info!("Ping!");
        Vec::new()
    }
}

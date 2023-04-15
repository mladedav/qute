pub(crate) struct PingHandler;

impl PingHandler {
    pub(crate) fn pong(&self) {
        tracing::info!("Pong!");
    }

    pub(crate) fn ping(&self) {
        tracing::info!("Ping!");
    }
}

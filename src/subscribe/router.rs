use std::convert::Infallible;

use mqttbytes::v5::Publish;
use tower::{util::BoxCloneService, Service, ServiceExt};

use super::handler::Handler;

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
            let service = router_match
                .value
                .ready()
                .await
                .expect("Error type is Infallible.");
            service
                .call(publish.clone())
                .await
                .expect("Error type is Infallible.");
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

use std::{collections::HashMap, convert::Infallible};

use mqttbytes::v5::Publish;
use tower::{util::BoxCloneService, Service, ServiceExt};

use crate::{ClientState, Handler};

use super::handler::{ErasedClientlessHandlerService, ErasedHandler};

enum RouteHandler<S> {
    WithoutState(Box<dyn ErasedHandler<S>>),
    WithState(Box<dyn ErasedClientlessHandlerService>),
}

impl<S: Clone> Clone for RouteHandler<S> {
    fn clone(&self) -> Self {
        match self {
            Self::WithoutState(handler) => Self::WithoutState(handler.clone_boxed()),
            Self::WithState(handler) => Self::WithState(handler.clone_boxed()),
        }
    }
}

pub struct HandlerRouterBuilder<S = ()> {
    routes: HashMap<String, RouteHandler<S>>,
}

impl<S> HandlerRouterBuilder<S> {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn add<const ASYNC: bool, M: Send + 'static>(
        &mut self,
        route: &str,
        handler: impl Handler<ASYNC, M, S> + 'static,
    ) where
        S: Clone + Send + 'static,
    {
        let erased = handler.erased();
        let without_state = RouteHandler::WithoutState(erased.clone_boxed());
        self.routes.insert(route.to_string(), without_state);
    }

    pub fn with_state<S2>(self, state: S) -> HandlerRouterBuilder<S2>
    where
        S: Clone + Send + 'static,
        S2: Clone + Send + 'static,
    {
        let mut routes = HashMap::<String, RouteHandler<S2>>::new();

        for (key, route) in self.routes {
            let route: RouteHandler<S2> = match route {
                RouteHandler::WithoutState(handler) => {
                    RouteHandler::WithState(Box::new(handler.with_state(state.clone())))
                }
                RouteHandler::WithState(service) => RouteHandler::WithState(service),
            };
            routes.insert(key, route);
        }

        HandlerRouterBuilder::<S2> { routes }
    }
}

impl HandlerRouterBuilder<()> {
    pub fn build(self) -> HandlerRouter {
        let mut new_routes = HashMap::new();

        for (key, route) in self.routes {
            let route = match route {
                RouteHandler::WithoutState(handler) => Box::new(handler.with_state(())),
                RouteHandler::WithState(service) => service,
            };
            new_routes.insert(key, route);
        }

        HandlerRouter { routes: new_routes }
    }
}

pub struct HandlerRouter {
    routes: HashMap<String, Box<dyn ErasedClientlessHandlerService>>,
}

impl HandlerRouter {
    pub(crate) fn build(self, client_state: ClientState) -> HandlerRouterWithClientState {
        let mut router = matchit::Router::new();

        for (key, route) in self.routes {
            router
                .insert(key, route.get_service(client_state.clone()))
                .unwrap();
        }

        HandlerRouterWithClientState { inner: router }
    }
}

pub struct HandlerRouterWithClientState {
    inner: matchit::Router<BoxCloneService<Publish, (), Infallible>>,
}

impl HandlerRouterWithClientState {
    pub async fn handle(&mut self, publish: Publish) {
        if let Ok(router_match) = self.inner.at_mut(&publish.topic) {
            let service = router_match.value;
            service.ready().await.expect("Error type is Infallible.");
            service
                .call(publish.clone())
                .await
                .expect("Error type is Infallible.");
        } else {
            tracing::debug!(topic = %publish.topic, "No matching route found.");
        }
    }
}

impl<S> Default for HandlerRouterBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

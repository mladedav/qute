use std::{
    convert::Infallible,
    future::{ready, Future, Ready},
    pin::Pin,
    task::{Context, Poll},
};

use mqttbytes::v5::Publish;
use tower::Service;

// use super::serivce::MqttService;

pub trait Handler<const ASYNC: bool, S = ()>: Clone + Send + Sized + 'static {
    type Future: Future<Output = ()> + Send + 'static;

    // Required method
    fn call(self, req: Publish, state: S) -> Self::Future;

    // Provided methods
    // TODO
    // fn layer<L, NewReqBody>(self, layer: L) -> Layered<L, Self, T, S, B, NewReqBody>
    // where
    //     L: Layer<HandlerService<Self, T, S, B>> + Clone,
    //     L::Service: Service<Request<NewReqBody>>,
    // {
    //     Layered {
    //         layer,
    //         handler: self,
    //         _marker: PhantomData,
    //     }
    // }

    /// Convert the handler into a [`Service`] by providing the state
    fn with_state(self, state: S) -> HandlerService<ASYNC, Self, S> {
        HandlerService::new(self, state)
    }
}

impl<F, S> Handler<false, S> for F
where
    F: FnOnce(Publish) + Clone + Send + 'static,
{
    type Future = Ready<()>;

    fn call(self, req: Publish, _state: S) -> Self::Future {
        self(req);
        ready(())
    }
}

impl<F, Fut, S> Handler<true, S> for F
where
    F: FnOnce(Publish) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, req: Publish, _state: S) -> Self::Future {
        Box::pin(async move {
            self(req).await;
        })
    }
}

pub struct HandlerService<const ASYNC: bool, H, S> {
    handler: H,
    state: S,
}

impl<const ASYNC: bool, H, S> Clone for HandlerService<ASYNC, H, S>
where
    H: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            state: self.state.clone(),
        }
    }
}

impl<const ASYNC: bool, H, S> HandlerService<ASYNC, H, S> {
    pub fn new(handler: H, state: S) -> Self {
        HandlerService { handler, state }
    }
}

impl<const A: bool, H, S> Service<Publish> for HandlerService<A, H, S>
where
    H: Handler<A, S> + Clone + Send + 'static,
    S: Clone + Send + 'static,
{
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    // type Future = TempFut;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Publish) -> Self::Future {
        let handler = self.handler.clone();
        let state = self.state.clone();
        Box::pin(async move {
            handler.call(req, state).await;
            Ok(())
        })
    }
}

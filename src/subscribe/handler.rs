use std::{
    convert::Infallible,
    future::{ready, Future, Ready},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use mqttbytes::v5::Publish;
use tower::Service;

// use super::serivce::MqttService;

pub trait Handler<const ASYNC: bool, M, S = ()>: Clone + Send + Sized + 'static
where
    S: Clone + Send,
{
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
    fn with_state(self, state: S) -> HandlerService<ASYNC, Self, M, S> {
        HandlerService::new(self, state)
    }
}

impl<F, S: Clone + Send> Handler<false, (), S> for F
where
    F: FnOnce(Publish) + Clone + Send + 'static,
{
    type Future = Ready<()>;

    fn call(self, req: Publish, _state: S) -> Self::Future {
        self(req);
        ready(())
    }
}

impl<F, S: Clone + Send> Handler<false, (S,), S> for F
where
    F: FnOnce(Publish, S) + Clone + Send + 'static,
{
    type Future = Ready<()>;

    fn call(self, req: Publish, state: S) -> Self::Future {
        self(req, state);
        ready(())
    }
}

impl<F, Fut, S: Clone + Send> Handler<true, (), S> for F
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

pub struct HandlerService<const ASYNC: bool, H, M, S> {
    handler: H,
    state: S,
    _phantom: PhantomData<M>,
}

impl<const ASYNC: bool, H, M, S> Clone for HandlerService<ASYNC, H, M, S>
where
    H: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            state: self.state.clone(),
            _phantom: self._phantom,
        }
    }
}

impl<const ASYNC: bool, H, M, S> HandlerService<ASYNC, H, M, S> {
    pub fn new(handler: H, state: S) -> Self {
        HandlerService {
            handler,
            state,
            _phantom: PhantomData,
        }
    }
}

impl<const A: bool, H, M, S> Service<Publish> for HandlerService<A, H, M, S>
where
    H: Handler<A, M, S> + Clone + Send + 'static,
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

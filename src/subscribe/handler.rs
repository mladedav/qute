use std::{
    convert::Infallible,
    future::{ready, Future, Ready},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use mqttbytes::v5::Publish;
use tower::Service;

use super::extractor::FromPublish;

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
    F: FnOnce() + Clone + Send + 'static,
{
    type Future = Ready<()>;

    fn call(self, _publish: Publish, _state: S) -> Self::Future {
        self();
        ready(())
    }
}

impl<F, T1, S: Clone + Send> Handler<false, (T1,), S> for F
where
    F: FnOnce(T1) + Clone + Send + 'static,
    T1: FromPublish<S>,
{
    type Future = Ready<()>;

    fn call(self, publish: Publish, state: S) -> Self::Future {
        let t1 = T1::from_publish(&publish, &state).unwrap();
        self(t1);
        ready(())
    }
}

impl<F, T1, T2, S: Clone + Send> Handler<false, (T1, T2), S> for F
where
    F: FnOnce(T1, T2) + Clone + Send + 'static,
    T1: FromPublish<S>,
    T2: FromPublish<S>,
{
    type Future = Ready<()>;

    fn call(self, publish: Publish, state: S) -> Self::Future {
        let t1 = T1::from_publish(&publish, &state).unwrap();
        let t2 = T2::from_publish(&publish, &state).unwrap();
        self(t1, t2);
        ready(())
    }
}

impl<F, T1, T2, T3, S: Clone + Send> Handler<false, (T1, T2, T3), S> for F
where
    F: FnOnce(T1, T2, T3) + Clone + Send + 'static,
    T1: FromPublish<S>,
    T2: FromPublish<S>,
    T3: FromPublish<S>,
{
    type Future = Ready<()>;

    fn call(self, publish: Publish, state: S) -> Self::Future {
        let t1 = T1::from_publish(&publish, &state).unwrap();
        let t2 = T2::from_publish(&publish, &state).unwrap();
        let t3 = T3::from_publish(&publish, &state).unwrap();
        self(t1, t2, t3);
        ready(())
    }
}

impl<F, Fut, S: Clone + Send> Handler<true, (), S> for F
where
    F: FnOnce(Publish) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, publish: Publish, _state: S) -> Self::Future {
        Box::pin(async move {
            self(publish).await;
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

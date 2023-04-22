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

pub trait Handler<const ASYNC: bool, M, S = ()>: Clone + Send + Sized + 'static
where
    S: Clone + Send,
{
    type Future: Future<Output = ()> + Send + 'static;

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

macro_rules! all_tuples {
    ($macro:ident) => {
        $macro!();
        $macro!(T1);
        $macro!(T1, T2);
        $macro!(T1, T2, T3);
        $macro!(T1, T2, T3, T4);
        $macro!(T1, T2, T3, T4, T5);
        $macro!(T1, T2, T3, T4, T5, T6);
        $macro!(T1, T2, T3, T4, T5, T6, T7);
        $macro!(T1, T2, T3, T4, T5, T6, T7, T8);
    };
}

macro_rules! impl_handler {
    (
        $($ty:ident),*
    ) => {
        impl<F, $($ty,)* S> Handler<false, ($($ty,)*), S> for F
        where
            S: Clone + Send,
            F: FnOnce($($ty,)*) + Clone + Send + 'static,
            $( $ty: FromPublish<S>, )*
        {
            type Future = Ready<()>;

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn call(self, publish: Publish, state: S) -> Self::Future {
                $(
                    let $ty = $ty::from_publish(&publish, &state).unwrap();
                )*
                self($($ty, )*);
                ready(())
            }
        }
    }
}

macro_rules! impl_async_handler {
    (
        $($ty:ident),*
    ) => {
        impl<F, $($ty,)* Fut, S> Handler<true, ($($ty,)*), S> for F
        where
            S: Clone + Send,
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = ()> + Send,
            $( $ty: FromPublish<S> + Send + 'static, )*
        {
            type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn call(self, publish: Publish, state: S) -> Self::Future {
                $(
                    let $ty = $ty::from_publish(&publish, &state).unwrap();
                )*
                Box::pin(async move {
                    self($($ty, )*).await;
                })
            }
        }
    }
}

all_tuples!(impl_handler);
all_tuples!(impl_async_handler);

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

impl<const ASYNC: bool, H, M, S> Service<Publish> for HandlerService<ASYNC, H, M, S>
where
    H: Handler<ASYNC, M, S> + Clone + Send + 'static,
    S: Clone + Send + 'static,
{
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

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

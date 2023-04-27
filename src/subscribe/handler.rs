use std::{
    convert::Infallible,
    future::{ready, Future, Ready},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use mqttbytes::v5::Publish;
use tower::Service;

use super::extractor::Extractable;

pub trait Handler<const ASYNC: bool, M, S = ()>: Clone + Send + Sized + 'static
where
    S: Clone + Send + 'static,
    M: 'static,
{
    type Future: Future<Output = ()> + Send + 'static;

    fn call(self, req: Publish, state: S) -> Self::Future;

    fn erased(self) -> Box<dyn ErasedHandler<S>> {
        let wrapper = HandlerWrapper::<ASYNC, Self, M, S> {
            handler: self,
            _phantom: PhantomData,
        };
        Box::new(wrapper)
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
            S: Clone + Send + 'static,
            F: FnOnce($($ty,)*) + Clone + Send + 'static,
            $( $ty: Extractable<S> + 'static, )*
        {
            type Future = Ready<()>;

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn call(self, publish: Publish, state: S) -> Self::Future {
                $(
                    let $ty = $ty::extract(&publish, &state).unwrap();
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
            S: Clone + Send + 'static,
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = ()> + Send,
            $( $ty: Extractable<S> + Send + 'static, )*
        {
            type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn call(self, publish: Publish, state: S) -> Self::Future {
                $(
                    let $ty = $ty::extract(&publish, &state).unwrap();
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

#[repr(transparent)]
struct HandlerWrapper<const ASYNC: bool, H, M, S> {
    handler: H,
    _phantom: PhantomData<fn(M, S)>,
}

impl<const ASYNC: bool, H: Clone, M, S> Clone for HandlerWrapper<ASYNC, H, M, S>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            _phantom: PhantomData,
        }
    }
}

pub trait CloneErasedHandler<S> {
    fn clone_boxed(&self) -> Box<dyn ErasedHandler<S>>;
}

pub trait ErasedHandler<S>: CloneErasedHandler<S> + Send {
    fn call(&mut self, req: Publish, state: S) -> Pin<Box<dyn Future<Output = ()> + Send>>;
    fn with_state(&self, state: S) -> HandlerService<S>;
}

impl<const ASYNC: bool, H, M, S> CloneErasedHandler<S> for HandlerWrapper<ASYNC, H, M, S>
where
    H: Handler<ASYNC, M, S> + Clone,
    S: Clone + Send + 'static,
    M: 'static,
{
    fn clone_boxed(&self) -> Box<dyn ErasedHandler<S>> {
        Box::new(self.clone())
    }
}

impl<const ASYNC: bool, H, M, S> ErasedHandler<S> for HandlerWrapper<ASYNC, H, M, S>
where
    H: Handler<ASYNC, M, S>,
    S: Clone + Send + 'static,
    M: 'static,
{
    fn call(&mut self, req: Publish, state: S) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(self.handler.clone().call(req, state))
    }

    fn with_state(&self, state: S) -> HandlerService<S> {
        HandlerService {
            handler: self.clone_boxed(),
            state,
        }
    }
}

pub struct HandlerService<S> {
    handler: Box<dyn ErasedHandler<S>>,
    state: S,
}

impl<S> Clone for HandlerService<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone_boxed(),
            state: self.state.clone(),
        }
    }
}

impl<S: Clone + Send + 'static> Service<Publish> for HandlerService<S> {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Publish) -> Self::Future {
        let mut handler = self.handler.clone_boxed();
        let state = self.state.clone();
        Box::pin(async move {
            handler.call(req, state).await;
            Ok(())
        })
    }
}

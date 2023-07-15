use std::convert::Infallible;
use std::fmt::Debug;

use bytes::Bytes;
use mqttbytes::v5::Publish;
use mqttbytes::QoS;
use serde::Deserialize;

use crate::client::ClientState;

pub trait Extractable<S>: Sized {
    type Rejection: Debug;

    fn extract(
        publish: &Publish,
        state: &S,
        client_state: &ClientState,
    ) -> Result<Self, Self::Rejection>;
}

pub struct State<S>(pub S);

pub trait FromState<S> {
    fn from_state(state: &S) -> Self;
}

impl<S: Clone> FromState<S> for S {
    fn from_state(state: &S) -> Self {
        state.clone()
    }
}

impl<S, T: FromState<S>> Extractable<S> for State<T> {
    type Rejection = Infallible;

    fn extract(
        _publish: &Publish,
        state: &S,
        _client_state: &ClientState,
    ) -> Result<Self, Self::Rejection> {
        Ok(State(T::from_state(state)))
    }
}

pub trait FromPublish: Sized {
    type Rejection: Debug;

    fn from_publish(publish: &Publish) -> Result<Self, Self::Rejection>;
}

impl<S, T> Extractable<S> for T
where
    T: FromPublish,
{
    type Rejection = T::Rejection;

    fn extract(
        publish: &Publish,
        _state: &S,
        _client_state: &ClientState,
    ) -> Result<Self, Self::Rejection> {
        T::from_publish(publish)
    }
}

impl FromPublish for Publish {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish) -> Result<Self, Self::Rejection> {
        Ok(publish.clone())
    }
}

pub struct Topic(pub String);

impl FromPublish for Topic {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish) -> Result<Self, Self::Rejection> {
        Ok(Topic(publish.topic.clone()))
    }
}

impl FromPublish for QoS {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish) -> Result<Self, Self::Rejection> {
        Ok(publish.qos)
    }
}

impl FromPublish for Bytes {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish) -> Result<Self, Self::Rejection> {
        Ok(publish.payload.clone())
    }
}

pub struct Json<T>(pub T);

impl<T> FromPublish for Json<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Rejection = serde_json::Error;

    fn from_publish(publish: &Publish) -> Result<Self, Self::Rejection> {
        serde_json::from_slice(&publish.payload).map(Json)
    }
}

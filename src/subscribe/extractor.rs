use std::convert::Infallible;
use std::fmt::Debug;

use bytes::Bytes;
use mqttbytes::v5::Publish;
use mqttbytes::QoS;
use serde::Deserialize;

pub trait FromPublish<S>: Sized {
    type Rejection: Debug;

    fn from_publish(publish: &Publish, state: &S) -> Result<Self, Self::Rejection>;
}

impl<S> FromPublish<S> for Publish {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(publish.clone())
    }
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

impl<S, T: FromState<S>> FromPublish<S> for State<T> {
    type Rejection = Infallible;

    fn from_publish(_publish: &Publish, state: &S) -> Result<Self, Self::Rejection> {
        Ok(State(T::from_state(state)))
    }
}

pub struct Topic(pub String);

impl<S> FromPublish<S> for Topic {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Topic(publish.topic.clone()))
    }
}

impl<S> FromPublish<S> for QoS {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(publish.qos)
    }
}

impl<S> FromPublish<S> for Bytes {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(publish.payload.clone())
    }
}

pub struct Json<T>(pub T);

impl<T, S> FromPublish<S> for Json<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Rejection = serde_json::Error;

    fn from_publish(publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        serde_json::from_slice(&publish.payload).map(Json)
    }
}

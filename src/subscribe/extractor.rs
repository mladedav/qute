use std::convert::Infallible;
use std::fmt::Debug;

use mqttbytes::v5::Publish;

pub trait FromPublish<S>: Sized {
    type Rejection: Debug;

    fn from_publish(publish: &Publish, state: &S) -> Result<Self, Self::Rejection>;
}

pub struct State<S>(pub S);

impl<S: Clone> FromPublish<S> for State<S> {
    type Rejection = Infallible;

    fn from_publish(_publish: &Publish, state: &S) -> Result<Self, Self::Rejection> {
        Ok(State(state.clone()))
    }
}

impl<S> FromPublish<S> for Publish {
    type Rejection = Infallible;

    fn from_publish(publish: &Publish, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(publish.clone())
    }
}

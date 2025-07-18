
use crate::{Context, state::State};

#[derive(Debug, Clone, Default)]
pub struct Request<S> {
    pub context: Context<S>,
    pub state: State,
}

pub trait FromRequest<S>: Sized {
    fn from_request(request: &Request<S>) -> Result<Self, crate::Error>;
}

impl<S> FromRequest<S> for State {
    fn from_request(request: &Request<S>) -> Result<Self, crate::Error> {
        Ok(request.state.clone())
    }
}

impl<S> FromRequest<S> for Context<S>
where
    S: Clone,
{
    fn from_request(request: &Request<S>) -> Result<Self, crate::Error> {
        Ok(request.context.clone())
    }
}

impl<S> FromRequest<S> for Request<S>
where
    S: Clone,
{
    fn from_request(request: &Request<S>) -> Result<Self, crate::Error> {
        Ok(request.clone())
    }
}

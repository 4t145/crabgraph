use std::{borrow::Cow, collections::HashSet, sync::Arc};

use crate::{node::NodeKey, utils::IntoSet, Request};

mod function;
pub use function::EdgeFunction;
pub trait Edge<S>: Send + Sync + 'static {
    fn next_nodes(&self, request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error>;
    fn neighbours(&self) -> HashSet<NodeKey>;
    fn description(&self) -> String;
}

impl<S> Edge<S> for NodeKey {
    fn next_nodes(&self, _request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error> {
        Ok(HashSet::from([self.clone()]))
    }
    fn neighbours(&self) -> HashSet<NodeKey> {
        HashSet::from([self.clone()])
    }
    fn description(&self) -> String {
        format!("To NodeKey({})", self)
    }
}

impl<S> Edge<S> for HashSet<NodeKey> {
    fn next_nodes(&self, _request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error> {
        Ok(self.clone())
    }
    fn neighbours(&self) -> HashSet<NodeKey> {
        self.clone()
    }
    fn description(&self) -> String {
        format!("To Nodekeys [{self:?}]",)
    }
}

pub trait IntoEdge<S, A> {
    fn into_edge(self) -> std::sync::Arc<dyn Edge<S>>;
}

impl<S> IntoEdge<S, ()> for std::sync::Arc<dyn Edge<S>> {
    fn into_edge(self) -> std::sync::Arc<dyn Edge<S>> {
        self
    }
}

pub struct ByIntoSet<A> {
    _marker: std::marker::PhantomData<A>,
}

impl<S, T, A> IntoEdge<S, ByIntoSet<A>> for T
where
    T: IntoSet<NodeKey, A>,
{
    fn into_edge(self) -> std::sync::Arc<dyn Edge<S>> {
        Arc::new(self.into_set()) as Arc<dyn Edge<S>>
    }
}

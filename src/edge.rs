use std::{collections::HashSet, sync::Arc};

use crate::{Request, node::NodeKey, utils::IntoSet};

mod function;
pub use function::EdgeFunction;
use futures::future::{BoxFuture, ready};
pub trait Edge<S>: Send + Sync + 'static {
    fn next_nodes(&self, request: &Request<S>)
    -> BoxFuture<Result<HashSet<NodeKey>, crate::Error>>;
    fn neighbours(&self) -> HashSet<NodeKey>;
    fn description(&self) -> String;
}

impl<S> Edge<S> for NodeKey {
    fn next_nodes(
        &self,
        _request: &Request<S>,
    ) -> BoxFuture<Result<HashSet<NodeKey>, crate::Error>> {
        Box::pin(ready(Ok(HashSet::from([self.clone()]))))
    }
    fn neighbours(&self) -> HashSet<NodeKey> {
        HashSet::from([self.clone()])
    }
    fn description(&self) -> String {
        format!("To NodeKey({})", self)
    }
}

impl<S> Edge<S> for HashSet<NodeKey> {
    fn next_nodes(
        &self,
        _request: &Request<S>,
    ) -> BoxFuture<Result<HashSet<NodeKey>, crate::Error>> {
        Box::pin(ready(Ok(self.clone())))
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

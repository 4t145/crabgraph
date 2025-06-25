use std::{collections::HashSet, sync::Arc};

use crate::{
    Request,
    node::{IntoNodeKeySet, NodeKey},
};

mod function;
pub use function::EdgeFunction;
pub trait Edge<S>: Send + Sync + 'static {
    fn next_nodes(&self, request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error>;
}

impl<S> Edge<S> for NodeKey {
    fn next_nodes(&self, _request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error> {
        Ok(HashSet::from([self.clone()]))
    }
}
impl<S> Edge<S> for HashSet<NodeKey> {
    fn next_nodes(&self, _request: &Request<S>) -> Result<HashSet<NodeKey>, crate::Error> {
        Ok(self.clone())
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

pub enum ByIntoNodeKeySet {}

impl<S, T> IntoEdge<S, ByIntoNodeKeySet> for T
where
    T: IntoNodeKeySet,
{
    fn into_edge(self) -> std::sync::Arc<dyn Edge<S>> {
        Arc::new(self.into_node_key_set()) as Arc<dyn Edge<S>>
    }
}

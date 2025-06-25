use std::{borrow::Cow, collections::HashSet, fmt::Display, ops::Deref, sync::Arc};

use futures::future::BoxFuture;

use crate::{Request, state::State};
mod function;
pub use function::NodeFunction;
pub trait Node<S>: Send + Sync + 'static {
    fn call(
        self: Arc<Self>,
        request: Request<S>,
    ) -> BoxFuture<'static, Result<State, crate::Error>>;
}

pub trait IntoNode<S, A> {
    fn into_node(self) -> Arc<dyn Node<S>>;
}

impl<S> IntoNode<S, ()> for Arc<dyn Node<S>> {
    fn into_node(self) -> Arc<dyn Node<S>> {
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum NodeKey {
    Named(Cow<'static, str>),
    Start,
    End,
}

impl Display for NodeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl NodeKey {
    pub const fn const_new(name: &'static str) -> Self {
        NodeKey::Named(Cow::Borrowed(name))
    }
}

impl Deref for NodeKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            NodeKey::Named(name) => name,
            NodeKey::Start => "@start",
            NodeKey::End => "@end",
        }
    }
}

impl From<&'static str> for NodeKey {
    fn from(val: &'static str) -> Self {
        NodeKey::Named(Cow::Borrowed(val))
    }
}

impl From<String> for NodeKey {
    fn from(val: String) -> Self {
        NodeKey::Named(Cow::Owned(val))
    }
}

impl From<Cow<'static, str>> for NodeKey {
    fn from(val: Cow<'static, str>) -> Self {
        NodeKey::Named(val)
    }
}

pub trait TryIntoNodeKeySet {
    fn try_into_node_key_set(self) -> Result<HashSet<NodeKey>, crate::Error>;
}

pub trait IntoNodeKeySet {
    fn into_node_key_set(self) -> HashSet<NodeKey>;
}

impl IntoNodeKeySet for NodeKey {
    fn into_node_key_set(self) -> HashSet<NodeKey> {
        HashSet::from([self])
    }
}

impl IntoNodeKeySet for &'static str {
    fn into_node_key_set(self) -> HashSet<NodeKey> {
        HashSet::from([self.into()])
    }
}

impl IntoNodeKeySet for Cow<'static, str> {
    fn into_node_key_set(self) -> HashSet<NodeKey> {
        HashSet::from([self.into()])
    }
}

impl<T: Into<NodeKey>> IntoNodeKeySet for Vec<T> {
    fn into_node_key_set(self) -> HashSet<NodeKey> {
        self.into_iter().map(|t| t.into()).collect()
    }
}

impl<const N: usize, T: Into<NodeKey>> IntoNodeKeySet for [T; N] {
    fn into_node_key_set(self) -> HashSet<NodeKey> {
        self.into_iter().map(|t| t.into()).collect()
    }
}

impl<T: Into<NodeKey>> IntoNodeKeySet for std::collections::HashSet<T> {
    fn into_node_key_set(self) -> HashSet<NodeKey> {
        self.into_iter().map::<NodeKey, _>(|t| t.into()).collect()
    }
}

impl<T: TryIntoNodeKeySet> TryIntoNodeKeySet for Result<T, crate::Error> {
    fn try_into_node_key_set(self) -> Result<HashSet<NodeKey>, crate::Error> {
        self.and_then(|s| s.try_into_node_key_set())
    }
}

impl<T: IntoNodeKeySet> TryIntoNodeKeySet for T {
    fn try_into_node_key_set(self) -> Result<HashSet<NodeKey>, crate::Error> {
        Ok(self.into_node_key_set())
    }
}

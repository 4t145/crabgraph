use std::{borrow::Cow, fmt::Display, ops::Deref, sync::Arc};

use futures::future::BoxFuture;

use crate::{Request, state::State};
mod function;
pub use function::NodeFunction;
mod sequence;
pub use sequence::NodeSequence;
pub trait Node<S>: Send + Sync + 'static {
    fn call(
        self: Arc<Self>,
        request: Request<S>,
    ) -> BoxFuture<'static, Result<modify::SendDynModification<State>, crate::Error>>;
}

impl<S> dyn Node<S>
where
    S: Send + Sync + Clone + 'static,
{
    pub fn then<N, A>(self: Arc<Self>, node: N) -> NodeSequence<S>
    where
        N: IntoNode<S, A>,
    {
        NodeSequence::new(vec![self, node.into_node()])
    }
    pub fn then_sequence(self: Arc<Self>, mut sequence: NodeSequence<S>) -> NodeSequence<S> {
        sequence.0 = vec![self].into_iter().chain(sequence.0).collect();
        sequence
    }
}

pub trait IntoNode<S, A> {
    fn into_node(self) -> Arc<dyn Node<S>>;
}

impl<S, N> IntoNode<S, ()> for Arc<N>
where
    N: Node<S>,
{
    fn into_node(self) -> Arc<dyn Node<S>> {
        self
    }
}

impl<S> IntoNode<S, ()> for Arc<dyn Node<S>> {
    fn into_node(self) -> Arc<dyn Node<S>> {
        self
    }
}

pub enum ByArc {}

impl<S, N> IntoNode<S, ByArc> for N
where
    N: Node<S>,
{
    fn into_node(self) -> Arc<dyn Node<S>> {
        Arc::new(self) as Arc<dyn Node<S>>
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

use std::sync::Arc;

use crate::{
    node::{IntoNode, Node},
    request::Request,
};
#[derive(Default, Clone)]
pub struct NodeSequence<S>(pub Vec<Arc<dyn Node<S>>>);

impl<S: Send + Sync + Clone + 'static> NodeSequence<S> {
    pub fn new(nodes: Vec<Arc<dyn Node<S>>>) -> Self {
        NodeSequence(nodes)
    }
    pub fn then<N, A>(mut self, node: N) -> NodeSequence<S>
    where
        N: IntoNode<S, A>,
    {
        self.0.push(node.into_node());
        self
    }
    pub fn then_sequence(mut self, sequence: NodeSequence<S>) -> NodeSequence<S> {
        self.0.extend(sequence.0);
        self
    }
}

impl<S> Node<S> for NodeSequence<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn call(
        self: Arc<Self>,
        request: crate::Request<S>,
    ) -> futures::future::BoxFuture<'static, Result<crate::state::State, crate::Error>> {
        let nodes = self.0.clone();
        Box::pin(async move {
            let context = request.context;
            let mut state = request.state;
            let len = nodes.len();
            for (idx, node) in nodes.into_iter().enumerate() {
                let request = Request {
                    context: context.clone(),
                    state: state.clone(),
                };
                let step_state = node.call(request).await?;
                if idx == len - 1 {
                    return Ok(step_state);
                } else {
                    state = Arc::new(step_state);
                }
            }
            Ok(state.as_ref().clone())
        })
    }
}

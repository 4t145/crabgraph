use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::{
    edge::{Edge, IntoEdge},
    node::{IntoNode, Node, NodeKey},
    request::Request,
    state::State,
};

pub mod edge;
pub mod node;
pub mod request;
pub mod state;
pub mod typed;
pub mod utils;
pub trait TransferObject: Sized + Serialize + DeserializeOwned + Send + Sync + 'static {}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Context<S> {
    pub state: S,
}

impl<S> Context<S>
where
    Self: Clone,
{
    pub fn new_request(&self, state: State) -> Request<S> {
        Request {
            context: self.clone(),
            state: Arc::new(state),
        }
    }
}

pub type JsonValue = serde_json::Value;

pub struct Response {
    pub state: JsonValue,
}
#[derive(Debug, Error)]
pub enum Error {
    #[error("Graph error: {0}")]
    GraphError(#[from] GraphError),
    #[error("Tokio join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Resolve next nodes for {node_key}: {error}")]
    ResolveNextNodesError {
        #[source]
        error: Box<Error>,
        node_key: NodeKey,
    },
}

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Node<S> {0} must have an out edge")]
    MissingOutEdge(NodeKey),
    #[error("Undefined node: {0}")]
    UndefinedNode(NodeKey),
    #[error("Undefined route: {0}")]
    UndefinedRoute(String),
    #[error("Next node cannot be Start")]
    PointToStart,
    #[error("Empty edge ({description}) from {from}")]
    EmptyEdge { from: NodeKey, description: String },
    #[error("Graph cannot reach End node")]
    UnreachableEndNode,
}

pub struct Graph<S> {
    pub nodes: HashMap<NodeKey, Arc<dyn Node<S>>>,
    pub edges: HashMap<NodeKey, Vec<Arc<dyn Edge<S>>>>,
}

impl<S> Default for Graph<S> {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }
}

impl<S> Clone for Graph<S> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        }
    }
}

impl<S> Graph<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_edge<N: Into<NodeKey>, E: IntoEdge<S, A>, A>(
        &mut self,
        from: N,
        edge: E,
    ) -> &mut Self {
        let from_key = from.into();
        self.edges
            .entry(from_key)
            .or_default()
            .push(edge.into_edge());
        self
    }
    pub fn add_node<K: Into<NodeKey>, N: IntoNode<S, A>, A>(
        &mut self,
        key: K,
        node: N,
    ) -> &mut Self {
        self.nodes.insert(key.into(), node.into_node());
        self
    }
    pub fn check(&self) -> Result<(), GraphError> {
        let mut checked = HashSet::new();
        let mut next_to_check = HashSet::new();
        next_to_check.insert(NodeKey::Start);
        loop {
            let mut new_next_to_check = HashSet::new();
            for node_key in next_to_check {
                if node_key == NodeKey::End {
                    checked.insert(node_key);
                    continue;
                }
                let edges = self
                    .edges
                    .get(&node_key)
                    .filter(|e| !e.is_empty())
                    .ok_or_else(|| GraphError::MissingOutEdge(node_key.clone()))?;
                let mut neighbours = HashSet::new();
                for edge in edges {
                    let neighbours_for_edge = edge.neighbours();
                    if neighbours_for_edge.is_empty() {
                        return Err(GraphError::EmptyEdge {
                            from: node_key.clone(),
                            description: edge.description(),
                        });
                    }
                    if neighbours_for_edge.contains(&NodeKey::Start) {
                        return Err(GraphError::PointToStart);
                    }
                    neighbours.extend(neighbours_for_edge);
                }
                checked.insert(node_key.clone());
                let unchecked_neighbours: HashSet<_> =
                    neighbours.difference(&checked).cloned().collect();
                if unchecked_neighbours.is_empty() {
                    continue;
                } else {
                    new_next_to_check.extend(unchecked_neighbours);
                }
            }
            next_to_check = new_next_to_check;
            if next_to_check.is_empty() {
                break;
            }
        }
        if !checked.contains(&NodeKey::End) {
            return Err(GraphError::UnreachableEndNode);
        }
        Ok(())
    }
    pub async fn run(self: Arc<Self>, request: Request<S>) -> Result<State, Error> {
        struct TaskCompleted {
            result: Result<State, Error>,
            node_key: NodeKey,
        }
        let mut task_set = tokio::task::JoinSet::new();
        task_set.spawn(futures::future::ready(
            // start trigger task
            TaskCompleted {
                result: Ok(request.state.as_ref().clone()),
                node_key: NodeKey::Start,
            },
        ));
        let mut output_state = State::default();
        loop {
            enum Event {
                TaskCompleted(TaskCompleted),
            }
            let event = tokio::select! {
                result = task_set.join_next(), if !task_set.is_empty() => {
                    Event::TaskCompleted(result.expect("not empty set")?)
                    // Handle the result of the completed task
                }
                else => {
                    // All tasks completed
                    break;
                }
            };
            match event {
                Event::TaskCompleted(TaskCompleted { result, node_key }) => {
                    let yield_state = Arc::new(result?);
                    let edges = self
                        .edges
                        .get(&node_key)
                        .filter(|e| !e.is_empty())
                        .ok_or_else(|| GraphError::MissingOutEdge(node_key.clone()))?;
                    tracing::info!(%node_key, ?yield_state, "Node completed ");
                    let request = Request {
                        state: yield_state.clone(),
                        context: request.context.clone(),
                    };
                    for e in edges {
                        for to_node_key in
                            e.next_nodes(&request)
                                .map_err(|e| Error::ResolveNextNodesError {
                                    error: Box::new(e),
                                    node_key: node_key.clone(),
                                })?
                        {
                            if to_node_key == NodeKey::End {
                                // merge result
                                output_state.merge(&yield_state);
                            } else {
                                let node = self.nodes.get(&to_node_key).ok_or_else(|| {
                                    GraphError::UndefinedNode(to_node_key.clone())
                                })?;
                                let fut = node.clone().call(Request {
                                    state: yield_state.clone(),
                                    context: request.context.clone(),
                                });
                                let node_key = to_node_key.clone();
                                task_set.spawn(async move {
                                    let result = fut.await;
                                    TaskCompleted { result, node_key }
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(output_state)
    }
    pub fn compile(self) -> Result<Arc<Self>, GraphError> {
        self.check()?;
        Ok(Arc::new(self))
    }
}

impl<S> Node<S> for Graph<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn call(self: Arc<Self>, request: Request<S>) -> BoxFuture<'static, Result<State, Error>> {
        Box::pin(async move { self.run(request).await })
    }
}

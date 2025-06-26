use std::sync::Arc;

use crate::node::Node;
fn node_api<S>(node: Arc<dyn Node<S>>) -> axum::Router {
    todo!()
}
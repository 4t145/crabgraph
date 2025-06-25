use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crabgraph::{
    Context, Graph,
    node::{Node, NodeKey},
    state::State,
    typed::json::Json,
};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default)]
pub struct App {
    countor: Arc<AtomicUsize>,
}
const INCREASE_COUNTER: NodeKey = NodeKey::const_new("increase_counter");
const ADD_LOG: NodeKey = NodeKey::const_new("add_log");
const PRINT_STATE: NodeKey = NodeKey::const_new("print_state");
#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let mut graph = crate::Graph::<App>::new();
    let context = Context::<App>::default();
    graph
        // nodes
        .add_node(ADD_LOG, add_log)
        .add_node(PRINT_STATE, print_state)
        .add_node(INCREASE_COUNTER, increase_counter)
        // edges
        .add_edge(ADD_LOG, [PRINT_STATE, NodeKey::End])
        .add_edge(INCREASE_COUNTER, print_if_odd)
        .add_edge(INCREASE_COUNTER, ADD_LOG)
        .add_edge(NodeKey::Start, [PRINT_STATE, INCREASE_COUNTER])
        .add_edge(PRINT_STATE, NodeKey::End);
    let graph = Arc::new(graph);
    let call_result_1 = graph
        .clone()
        .call(context.new_request(Default::default()))
        .await?;
    println!("Call result: {:?}", call_result_1);
    let call_result_2 = graph
        .clone()
        .call(context.new_request(Default::default()))
        .await?;
    println!("Call result: {:?}", call_result_2);
    Ok(())
}

async fn increase_counter(
    context: Context<App>,
    mut state: State,
) -> Result<State, crabgraph::Error> {
    let index = context.state.countor.fetch_add(1, Ordering::SeqCst);
    state.insert("index".to_string(), index.into());
    Ok(state)
}

async fn add_log(mut state: State) -> Result<State, crabgraph::Error> {
    state.insert("__log".to_string(), serde_json::json!("hello world"));
    Ok(state)
}

async fn print_state(state: State) -> Result<State, crabgraph::Error> {
    println!("State: {:?}", state);
    Ok(state)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Index {
    index: usize,
}
fn print_if_odd(Json(Index { index }): Json<Index>) -> NodeKey {
    if index % 2 == 1 {
        PRINT_STATE
    } else {
        NodeKey::End
    }
}

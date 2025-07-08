use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crabgraph::{
    Context, Graph, JsonObject, map,
    node::{IntoNode, Node, NodeKey},
    state::State,
    typed::json::TypedState,
};

use modify::{ModificationLayerExt, apply, call};
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
        .add_edge(
            INCREASE_COUNTER,
            (
                print_if_odd,
                map! {
                    "even" => PRINT_STATE,
                    "odd" => NodeKey::End
                },
            ),
        )
        .add_edge(INCREASE_COUNTER, ADD_LOG)
        .add_edge(NodeKey::Start, [PRINT_STATE, INCREASE_COUNTER])
        .add_edge(PRINT_STATE, NodeKey::End);
    let graph = graph.compile()?;
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
    let mut super_graph = crate::Graph::<App>::new();
    super_graph
        .add_node("child", graph.into_node().then(increase_counter))
        .add_edge(NodeKey::Start, "child")
        .add_edge("child", NodeKey::End);
    let super_graph = super_graph.compile()?;
    let call_result_3 = super_graph
        .call(context.new_request(Default::default()))
        .await?;
    println!("Call result: {:?}", call_result_3);
    Ok(())
}

async fn increase_counter(context: Context<App>, state: State) -> Result<(), crabgraph::Error> {
    let index = context.state.countor.fetch_add(1, Ordering::SeqCst);
    state
        .apply_modification(
            apply(call(|object: &mut JsonObject| {
                if !object.contains_key("index") {
                    object.insert("index".to_string(), serde_json::json!(null));
                }
            }))
            .then(modify::index("index"))
            .then_apply(modify::set(serde_json::json!(index))),
        )
        .await;
    Ok(())
}

async fn add_log(state: State) -> Result<(), crabgraph::Error> {
    state
        .apply_modification(
            apply(call(|object: &mut JsonObject| {
                if !object.contains_key("__log") {
                    object.insert("__log".to_string(), serde_json::json!(null));
                }
            }))
            .then(modify::index("__log"))
            .then_apply(modify::set(serde_json::json!("hello world"))),
        )
        .await;
    Ok(())
}

async fn print_state(state: State) -> Result<(), crabgraph::Error> {
    println!("State: {:?}", state);
    Ok(())
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Index {
    index: usize,
}
async fn print_if_odd(state: State) -> &'static str {
    let index = state
        .fetch_view(TypedState::<Index>::new())
        .await
        .unwrap_or_default()
        .index;

    if index % 2 == 1 { "odd" } else { "even" }
}

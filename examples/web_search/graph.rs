use std::{collections::HashMap, sync::Arc};

use crabgraph::{Graph, NodeError, node::NodeKey, state::State, typed::json::Json};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Tool};

use pyo3::{
    Bound, PyResult,
    types::{PyAnyMethods, PyDictMethods},
};
use serde::{Deserialize, Serialize};

use crate::{
    App, Config,
    prompts::{Prompt, QueryWriter, WebSearch},
    py_genai_client::PyGenaiClient,
    state::{OverallState, QueryGenerationState, WebSearchState},
};
const GENERATE_QUERY: NodeKey = NodeKey::const_new("generate_query");
const WEB_SEARCH: NodeKey = NodeKey::const_new("web_search");

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchQueryList {
    /// A list of search queries to be used for web research.
    queries: Vec<String>,
    /// A brief explanation of why these queries are relevant to the research topic.
    rationale: String,
}

async fn generate_query(
    Json(state): Json<OverallState>,
    config: Arc<Config>,
    llm: genai::Client,
) -> Result<Json<QueryGenerationState>, NodeError> {
    let number_queries = state
        .initial_search_query_count
        .unwrap_or(config.number_of_initial_queries);
    let schema: serde_json::Value = schemars::schema_for!(QueryGenerationState).into();
    let chat_option = ChatOptions::default()
        .with_response_format(schema)
        .with_temperature(1.0);
    let prompt = QueryWriter {
        research_topic: &state.get_research_topic(),
        number_queries,
        current_date: &chrono::Utc::now().to_rfc3339(),
    }
    .format_prompt();
    let response = llm
        .exec_chat(
            &config.query_generator_model,
            ChatRequest::default().append_message(ChatMessage::user(prompt)),
            Some(&chat_option),
        )
        .await?
        .into_first_text()
        .unwrap_or_default();
    let response = serde_json::from_str::<QueryGenerationState>(&response)?;
    Ok(Json(response))
}

async fn web_research(
    Json(state): Json<QueryGenerationState>,
    llm: genai::Client,
    py_llm: PyGenaiClient,
    config: Arc<Config>,
) -> Result<State, NodeError> {
    let total_query_count = state.querys.len();
    for (idx, query) in state.querys.into_iter().enumerate() {
        tracing::info!(
            "Web search for query ({idx}/{total_query_count}): {}",
            query.query
        );
        let chat_response = py_llm
            .generate_content(
                &config.query_generator_model,
                WebSearch {
                    research_topic: &query.query,
                    current_date: &chrono::Utc::now().to_rfc3339(),
                }
                .format_prompt(),
                |py| {
                    let args = pyo3::types::PyDict::new(py);
                    let google_search_tool = pyo3::types::PyDict::new(py);
                    google_search_tool.set_item("google_search", pyo3::types::PyDict::new(py))?;
                    let tools = pyo3::types::PyList::new(py, [google_search_tool])?;
                    args.set_item("tools", tools)?;
                    args.set_item("temperature", 0.0)?;
                    Ok(args)
                },
            )
            .await?;
        // pyo3::Python::with_gil(|py| {
        //     let urls_to_resolve = chat_response
        //         .into_bound(py)
        //         .get_item("candidates")?
        //         .get_item(0)?
        //         .get_item("grounding_metadata")?
        //         .get_item("grounding_chunks")?
        //         .extract::<Bound<'_, pyo3::types::PyList>>()?;
        //     let id = idx;
        //     let mut resolved_map = HashMap::new();
        //     for (idx, url) in urls_to_resolve.into_iter().enumerate() {
        //         let url_str: String = url.get_item("web")?.get_item("uri")?.extract()?;
        //         if !resolved_map.contains_key(&url_str) {
        //             resolved_map.insert(
        //                 url_str,
        //                 format!("https://vertexaisearch.cloud.google.com/id/{id}-{idx}"),
        //             );
        //         }
        //     }
        //     tracing::info!("response: {:?}", chat_response);
        //     PyResult::Ok(())
        // });
        tracing::info!(
            "Web search response for query ({idx}/{total_query_count}): {:?}",
            chat_response
        );
    }
    Ok(State::default())
}
pub async fn graph() -> Result<Arc<Graph<App>>, crabgraph::Error> {
    let mut graph = Graph::<App>::new();
    graph
        .add_node(GENERATE_QUERY, generate_query)
        .add_node(WEB_SEARCH, web_research);
    graph
        .add_edge(NodeKey::Start, GENERATE_QUERY)
        .add_edge(GENERATE_QUERY, WEB_SEARCH)
        .add_edge(WEB_SEARCH, NodeKey::End);
    let graph = graph.compile()?;
    Ok(graph)
}

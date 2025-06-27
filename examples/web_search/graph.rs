use std::sync::Arc;

use crabgraph::{Graph, NodeError, node::NodeKey, typed::json::Json};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Tool};
use rig::{
    client::ProviderClient,
    completion::{Chat, Completion},
};
use serde::{Deserialize, Serialize};

use crate::{
    App, Config,
    prompts::{Prompt, QueryWriter},
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
    .prompt();
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
    state: Json<WebSearchState>,
    llm: genai::Client,
    config: Arc<Config>,
) -> Result<Json<OverallState>, NodeError> {
    llm.exec_chat(
        &config.query_generator_model,
        ChatRequest::default()
            .append_message(ChatMessage::user(prompt))
            .with_tools(vec![Tool {
                name: todo!(),
                description: todo!(),
                schema: todo!(),
            }]),
        ChatOptions::default().with_too,
    );
    Ok(todo!())
}
pub fn graph() -> Result<Arc<Graph<App>>, crabgraph::Error> {
    let openai_client = rig::providers::gemini::Client::from_env();

    let mut graph = Graph::<App>::new();
    graph.add_node(GENERATE_QUERY, generate_query);
    graph.add_edge(NodeKey::Start, GENERATE_QUERY);
    graph.add_edge(GENERATE_QUERY, NodeKey::End);
    let graph = graph.compile()?;
    Ok(graph)
}

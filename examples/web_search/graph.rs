use std::{collections::HashMap, sync::Arc};

use crabgraph::{Graph, NodeError, node::NodeKey, state::State, typed::json::Json};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, JsonSpec, Tool};

use serde::{Deserialize, Serialize};

use crate::{
    App, Config,
    prompts::{Prompt, QueryWriter, WebSearch},
    state::{OverallState, QueryGenerationState, WebSearchState},
    utils,
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
    let chat_option = ChatOptions::default()
        .with_response_format(JsonSpec::new(
            "QueryGenerationState",
            utils::schema::<QueryGenerationState>(),
        ))
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
    config: Arc<Config>,
) -> Result<OverallState, NodeError> {
    let total_query_count = state.querys.len();
    for (idx, query) in state.querys.into_iter().enumerate() {
        tracing::info!(
            "Web search for query ({idx}/{total_query_count}): {}",
            query.query
        );
        let chat_response = llm
            .exec_chat(
                &config.query_generator_model,
                ChatRequest::new(vec![ChatMessage::user(
                    WebSearch {
                        research_topic: &query.query,
                        current_date: &chrono::Utc::now().to_rfc3339(),
                    }
                    .format_prompt(),
                )])
                .with_tools(vec![
                    Tool::new("googleSearch").with_config(serde_json::json!({})),
                ]),
                Some(&ChatOptions::default().with_temperature(0.0)),
            )
            .await?;
        let resolved_urls = utils::resolve_urls(
            chat_response.raw_body["candidates"][0]["groundingMetadata"]["groundingChunks"].clone(),
            idx,
        );
        let citations = utils::get_citations(&chat_response.raw_body, &resolved_urls);
        let modified_text = utils::insert_citation_markers(
            chat_response.first_text().unwrap_or_default(),
            &citations,
        );
        let sources_gathered = citations
            .into_iter()
            .map(|item| item.segments)
            .flatten()
            .filter_map(|segment| serde_json::to_value(segment).ok())
            .collect::<Vec<_>>();
        tracing::info!(?resolved_urls);
        tracing::info!(
            "Web search response for query ({idx}/{total_query_count}): {}",
            chat_response.raw_body.to_string()
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

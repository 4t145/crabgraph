use std::{collections::HashMap, sync::Arc};

use crabgraph::{Graph, NodeError, map, node::NodeKey, state::State, typed::json::TypedState};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, JsonSpec, Tool};

use serde::{Deserialize, Serialize};

use crate::{
    App, Config,
    prompts::{AnswerInstructions, Prompt, QueryWriter, Reflection, WebSearch},
    state::{
        AddLoopCount, OverallState, OverallStateUpdate, QueryGenerationState, ReflectionState,
        ReflectionStateUpdate, WebSearchState,
    },
    utils,
};
const GENERATE_QUERY: NodeKey = NodeKey::const_new("generate_query");
const WEB_SEARCH: NodeKey = NodeKey::const_new("web_search");
const REFLECTION: NodeKey = NodeKey::const_new("reflection");
const FINALIZE_ANSWER: NodeKey = NodeKey::const_new("finalize_answer");

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchQueryList {
    /// A list of search queries to be used for web research.
    queries: Vec<String>,
    /// A brief explanation of why these queries are relevant to the research topic.
    rationale: String,
}

async fn generate_query(
    state: State,
    config: Arc<Config>,
    llm: genai::Client,
) -> Result<(), NodeError> {
    let overall_state = state.fetch_view(TypedState::<OverallState>::new()).await?;
    let number_queries = overall_state.initial_search_query_count;
    let chat_option = ChatOptions::default()
        .with_response_format(JsonSpec::new(
            "QueryGenerationState",
            utils::schema::<QueryGenerationState>(),
        ))
        .with_temperature(1.0);
    let prompt = QueryWriter {
        research_topic: &overall_state.get_research_topic(),
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
    state.apply_modification(response).await;
    Ok(())
}

async fn web_research(
    state: State,
    llm: genai::Client,
    config: Arc<Config>,
) -> Result<(), NodeError> {
    let query_state = state
        .fetch_view(TypedState::<QueryGenerationState>::new())
        .await?;
    let total_query_count = query_state.search_query.len();
    for (idx, query) in query_state.search_query.into_iter().enumerate() {
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
                Some(
                    &ChatOptions::default()
                        .with_temperature(0.0)
                        .with_capture_raw_body(true),
                ),
            )
            .await?;
        let raw_body = chat_response.captured_raw_body.as_ref().unwrap();
        let resolved_urls = utils::resolve_urls(
            raw_body["candidates"][0]["groundingMetadata"]["groundingChunks"].clone(),
            idx,
        );
        let citations = utils::get_citations(&raw_body, &resolved_urls);
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

        state
            .apply_modification(OverallStateUpdate {
                sources_gathered,
                web_research_result: vec![serde_json::json!(modified_text)],
                ..Default::default()
            })
            .await;
        tracing::info!("Web search response for query ({idx}/{total_query_count})",);
    }
    Ok(())
}

async fn reflection(
    state: State,
    config: Arc<Config>,
    llm: genai::Client,
) -> Result<(), NodeError> {
    let overall_state = state.fetch_view(TypedState::<OverallState>::new()).await?;
    tracing::info!("Starting reflection process...");

    let chat_option = ChatOptions::default()
        .with_response_format(JsonSpec::new(
            "ReflectionState",
            utils::schema::<ReflectionState>(),
        ))
        .with_temperature(1.0);

    let prompt = Reflection {
        research_topic: &overall_state.get_research_topic(),
        summaries: &overall_state
            .web_research_result
            .iter()
            .filter_map(|x| serde_json::to_string(x).ok())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"),
    }
    .format_prompt();

    let response = llm
        .exec_chat(
            &config.reflection_model,
            ChatRequest::default().append_message(ChatMessage::user(prompt)),
            Some(&chat_option),
        )
        .await?
        .into_first_text()
        .unwrap_or_default();

    let reflection_result = serde_json::from_str::<ReflectionState>(&response)?;

    state
        .apply_modification((
            AddLoopCount,
            ReflectionStateUpdate {
                is_sufficient: reflection_result.is_sufficient,
                knowledge_gap: reflection_result.knowledge_gap,
                follow_up_queries: reflection_result.follow_up_queries,
                number_of_ran_queries: overall_state.search_query.len(),
                ..Default::default()
            },
        ))
        .await;

    Ok(())
}

async fn evaluate_research(state: State, config: Arc<Config>) -> Result<NodeKey, NodeError> {
    let overall_state = state.fetch_view(TypedState::<OverallState>::new()).await?;
    let max_research_loops = config.max_research_loops;
    tracing::info!(
        "Evaluating research: is_sufficient: {}, research_loop_count: {}, max_research_loops: {}",
        overall_state.is_sufficient,
        overall_state.research_loop_count,
        max_research_loops
    );
    if overall_state.is_sufficient || overall_state.research_loop_count >= max_research_loops {
        Ok(FINALIZE_ANSWER)
    } else {
        // TODO: Implement parallel web research for follow-up queries
        Ok(WEB_SEARCH)
    }
}

async fn finalize_answer(
    state: State,
    config: Arc<Config>,
    llm: genai::Client,
) -> Result<(), NodeError> {
    let overall_state = state.fetch_view(TypedState::<OverallState>::new()).await?;
    tracing::info!("Finalizing answer...");
    let chat_option = ChatOptions::default();

    let prompt = AnswerInstructions {
        research_topic: &overall_state.get_research_topic(),
        summaries: &overall_state
            .web_research_result
            .iter()
            .filter_map(|x| serde_json::to_string(x).ok())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"),
        current_date: &chrono::Utc::now().to_rfc3339(),
    }
    .format_prompt();

    let mut response = llm
        .exec_chat(
            &config.answer_model,
            ChatRequest::default().append_message(ChatMessage::user(prompt)),
            Some(&chat_option),
        )
        .await?
        .into_first_text()
        .unwrap_or_default();

    // Replace short URLs with original URLs and gather unique sources
    let mut unique_source = vec![];
    for source in overall_state.sources_gathered {
        if let Some(short_url) = source.get("short_url").and_then(|s| s.as_str()) {
            if let Some(value) = source.get("value").and_then(|s| s.as_str()) {
                if response.contains(short_url) {
                    response = response.replace(short_url, value);
                    unique_source.push(source);
                }
            }
        }
    }

    state
        .apply_modification(OverallStateUpdate {
            messages: vec![crate::model::Message::ai(response)],
            sources_gathered: unique_source,
            ..Default::default()
        })
        .await;

    Ok(())
}

pub async fn graph() -> Result<Arc<Graph<App>>, crabgraph::Error> {
    let mut graph = Graph::<App>::new();
    graph
        .add_node(GENERATE_QUERY, generate_query)
        .add_node(WEB_SEARCH, web_research)
        .add_node(REFLECTION, reflection)
        .add_node(FINALIZE_ANSWER, finalize_answer);

    graph
        .add_edge(NodeKey::Start, GENERATE_QUERY)
        .add_edge(GENERATE_QUERY, WEB_SEARCH)
        .add_edge(WEB_SEARCH, REFLECTION)
        .add_edge(
            REFLECTION,
            (
                evaluate_research,
                map! {
                    FINALIZE_ANSWER => FINALIZE_ANSWER,
                    WEB_SEARCH => WEB_SEARCH
                },
            ),
        )
        .add_edge(FINALIZE_ANSWER, NodeKey::End);

    let graph = graph.compile()?;
    Ok(graph)
}

use std::sync::Arc;

use crabgraph::{Context, request::FromRequest, state::State, typed::json::JsonValueView};
use genai::{ModelIden, adapter::AdapterKind, resolver::AuthData};

use serde::{Deserialize, Serialize};

use crate::{graph::graph, model::Message, state::OverallState};
mod graph;
mod model;
mod prompts;
// mod py_genai_client;
mod state;
mod utils;
// Graph
#[derive(Debug, Clone)]
pub struct App {
    config: Arc<Config>,
    llm: genai::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    query_generator_model: String,
    reflection_model: String,
    answer_model: String,
    number_of_initial_queries: u32,
    max_research_loops: u32,
}

impl FromRequest<App> for Arc<Config> {
    fn from_request(request: &crabgraph::request::Request<App>) -> Result<Self, crabgraph::Error> {
        let config = request.context.state.config.clone();
        Ok(config)
    }
}

impl FromRequest<App> for genai::Client {
    fn from_request(request: &crabgraph::request::Request<App>) -> Result<Self, crabgraph::Error> {
        let llm = request.context.state.llm.clone();
        Ok(llm)
    }
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(Config {
        query_generator_model: "gemini-2.0-flash".to_string(),
        reflection_model: "gemini-2.0-flash".to_string(),
        answer_model: "gemini-2.0-flash".to_string(),
        number_of_initial_queries: 3,
        max_research_loops: 1,
    });
    let app = App {
        config: config.clone(),
        llm: genai::Client::builder()
            .with_auth_resolver_fn(|iden: ModelIden| {
                if matches!(iden.adapter_kind, AdapterKind::Gemini) {
                    Ok(Some(AuthData::from_env("GEMINI_API_KEY")))
                } else {
                    Ok(None)
                }
            })
            .build(),
    };

    let context = Context { state: app };
    let graph = graph().await?;
    let request = context.new_request(State::from_typed(OverallState {
        messages: vec![Message::human("请问中国境内目前有哪些生产ddr4内存的厂商？")],
        initial_search_query_count: config.number_of_initial_queries,
        max_research_loops: config.max_research_loops,
        research_loop_count: 0,
        reasoning_model: config.reflection_model.clone(),
        ..Default::default()
    })?);
    graph.run(request.clone()).await?;
    let result = request.state.fetch_view(JsonValueView).await;
    let value_to_string_pretty = serde_json::to_string_pretty(&result)?;
    tracing::info!("Graph execution completed {value_to_string_pretty}");
    Ok(())
}

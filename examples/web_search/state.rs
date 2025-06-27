use serde::{Deserialize, Serialize};

use crate::model::Message;


#[derive(Debug, Serialize, Deserialize)]
pub struct OverallState {
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub initial_search_query_count: Option<u32>,
    #[serde(default)]
    pub max_research_loops: Option<u32>,
    #[serde(default)]
    pub research_loop_count: Option<u32>,
    #[serde(default)]
    pub reasoning_model: String,
}

impl OverallState {
    pub fn get_research_topic(&self) -> String {
        Message::get_research_topic(&self.messages)
    }
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Query {
    /// A list of search queries to be used for web research.
    pub query: String,
    /// A brief explanation of why these queries are relevant to the research topic.
    pub rationale: String,
}
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QueryGenerationState {
    pub querys: Vec<Query>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchState {
    pub search_query: String,
    pub id: String,
}

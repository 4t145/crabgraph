use std::ops::Add;

use crabgraph::{JsonObject, JsonValue};
use modify::Modification;
use modify_json::{
    ensure::{array_field, boolean_field, field, number_field, string_field},
    serialize::{ExtendJsonArray, SetJson},
};
use serde::{Deserialize, Serialize};

use crate::model::Message;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OverallState {
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub initial_search_query_count: u32,
    #[serde(default)]
    pub max_research_loops: u32,
    #[serde(default)]
    pub research_loop_count: u32,
    #[serde(default)]
    pub reasoning_model: String,
    #[serde(default)]
    pub sources_gathered: Vec<JsonValue>,
    #[serde(default)]
    pub web_research_result: Vec<JsonValue>,
    #[serde(default)]
    pub search_query: Vec<JsonValue>,
    #[serde(default)]
    pub is_sufficient: bool,
    #[serde(default)]
    pub knowledge_gap: String,
    #[serde(default)]
    pub follow_up_queries: Vec<String>,
    #[serde(default)]
    pub number_of_ran_queries: usize,
    #[serde(default)]
    pub final_answer: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Modification)]
#[modify(target = "crabgraph::JsonObject")]
pub struct OverallStateUpdate {
    #[serde(default)]
    #[modify(by = array_field("messages").then(ExtendJsonArray))]
    pub messages: Vec<Message>,
    #[serde(default)]
    #[modify(by = string_field("reasoning_model").then(Set))]
    pub reasoning_model: String,
    #[serde(default)]
    #[modify(by = array_field("sources_gathered").then(Extend))]
    pub sources_gathered: Vec<JsonValue>,
    #[serde(default)]
    #[modify(by = array_field("web_research_result").then(Extend))]
    pub web_research_result: Vec<JsonValue>,
    #[serde(default)]
    #[modify(by = array_field("search_query").then(Extend))]
    pub search_query: Vec<JsonValue>,
}

pub struct AddLoopCount;

impl Modification<JsonObject> for AddLoopCount {
    fn modify(self, value: &mut JsonObject) {
        value
            .entry("research_loop_count")
            .and_modify(|v| {
                if let JsonValue::Number(num) = v {
                    if let Some(current) = num.as_u64() {
                        *num = serde_json::Number::from(current.add(1));
                    }
                }
            })
            .or_insert(JsonValue::Number(1.into()));
    }
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
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema, Modification)]
#[modify(target = "crabgraph::JsonObject")]
pub struct QueryGenerationState {
    #[modify(by = array_field("search_query").then(ExtendJsonArray))]
    pub search_query: Vec<Query>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchState {
    pub search_query: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReflectionState {
    /// Whether the current research is sufficient to answer the question
    pub is_sufficient: bool,
    /// Description of knowledge gaps that need to be filled
    pub knowledge_gap: String,
    /// List of follow-up queries to address knowledge gaps
    pub follow_up_queries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Modification)]
#[modify(target = "crabgraph::JsonObject")]
pub struct ReflectionStateUpdate {
    #[serde(default)]
    #[modify(by = boolean_field("is_sufficient").then(Set))]
    pub is_sufficient: bool,
    #[serde(default)]
    #[modify(by = string_field("knowledge_gap").then(Set))]
    pub knowledge_gap: String,
    #[serde(default)]
    #[modify(by = array_field("follow_up_queries").then(ExtendJsonArray))]
    pub follow_up_queries: Vec<String>,
    #[serde(default)]
    #[modify(by = field("number_of_ran_queries").then(SetJson))]
    pub number_of_ran_queries: usize,
    #[serde(default)]
    #[modify(by = string_field("final_answer").then(Set))]
    pub final_answer: String,
}

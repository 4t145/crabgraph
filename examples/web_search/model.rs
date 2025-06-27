use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Ai(AiMessage),
    Human(HumanMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiMessage {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HumanMessage {
    pub content: String,
}

impl Message {
    pub fn content(&self) -> &str {
        match self {
            Message::Ai(ai_message) => &ai_message.content,
            Message::Human(human_message) => &human_message.content,
        }
    }
    pub fn ai(content: impl Into<String>) -> Self {
        Message::Ai(AiMessage {
            content: content.into(),
        })
    }
    pub fn human(content: impl Into<String>) -> Self {
        Message::Human(HumanMessage {
            content: content.into(),
        })
    }
}
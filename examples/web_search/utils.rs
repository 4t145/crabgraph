use crate::model::Message;

impl Message {
    pub fn get_research_topic(list: &[Self]) -> String {
        if list.len() == 1 {
            return list[0].content().to_owned();
        } else {
            return list.iter().fold(String::new(), |mut s, m| {
                match m {
                    Message::Ai(ai_message) => {
                        s.push_str(&format!("Assistant: {}\n", ai_message.content));
                    }
                    Message::Human(human_message) => {
                        s.push_str(&format!("User: {}\n", human_message.content));
                    }
                }
                s
            });
        }
    }
}

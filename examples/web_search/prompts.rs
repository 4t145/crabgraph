pub trait Prompt {
    fn format_prompt(self) -> String;
}

pub struct QueryWriter<'a> {
    pub research_topic: &'a str,
    pub number_queries: u32,
    pub current_date: &'a str,
}

impl Prompt for QueryWriter<'_> {
    fn format_prompt(self) -> String {
        let QueryWriter {
            research_topic,
            number_queries,
            current_date,
        } = self;
        format!(
            r#"Your goal is to generate sophisticated and diverse web search queries. These queries are intended for an advanced automated web research tool capable of analyzing complex results, following links, and synthesizing information.

Instructions:
- Always prefer a single search query, only add another query if the original question requests multiple aspects or elements and one query is not enough.
- Each query should focus on one specific aspect of the original question.
- Don't produce more than {number_queries} queries.
- Queries should be diverse, if the topic is broad, generate more than 1 query.
- Don't generate multiple similar queries, 1 is enough.
- Query should ensure that the most current information is gathered. The current date is {current_date}.

Format: 
- Format your response as a JSON object with ALL two of these exact keys:
   - "rationale": Brief explanation of why these queries are relevant
   - "query": A list of search queries

Example:

Topic: What revenue grew more last year apple stock or the number of people buying an iphone
```json
{{
    "rationale": "To answer this comparative growth question accurately, we need specific data points on Apple's stock performance and iPhone sales metrics. These queries target the precise financial information needed: company revenue trends, product-specific unit sales figures, and stock price movement over the same fiscal period for direct comparison.",
    "query": ["Apple total revenue growth fiscal year 2024", "iPhone unit sales growth fiscal year 2024", "Apple stock price growth fiscal year 2024"],
}}
```

Context: {research_topic}
"#
        )
    }
}

pub struct WebSearch<'a> {
    pub research_topic: &'a str,
    pub current_date: &'a str,
}

impl Prompt for WebSearch<'_> {
    fn format_prompt(self) -> String {
        let WebSearch {
            research_topic,
            current_date,
        } = self;
        format!(
            r#"Conduct targeted Google Searches to gather the most recent, credible information on "{research_topic}" and synthesize it into a verifiable text artifact.

Instructions:
- Query should ensure that the most current information is gathered. The current date is {current_date}.
- Conduct multiple, diverse searches to gather comprehensive information.
- Consolidate key findings while meticulously tracking the source(s) for each specific piece of information.
- The output should be a well-written summary or report based on your search findings. 
- Only include the information found in the search results, don't make up any information.

Research Topic:
{research_topic}
"#,
        )
    }
}
